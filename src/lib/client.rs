use base64::{Engine, prelude::BASE64_STANDARD};
use futures::future::join_all;
use rquest::{
    Client, ClientBuilder, Proxy, RequestBuilder,
    header::{COOKIE, ORIGIN, REFERER},
    multipart::{Form, Part},
};
use rquest_util::Emulation;
use serde_json::Value;
use std::sync::LazyLock;
use tracing::warn;

use crate::{config::ENDPOINT, types::message::ImageSource};

/// The client to be used for requests to the Claude.ai
/// This client is used for requests that require a specific emulation
pub static SUPER_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    ClientBuilder::new()
        .emulation(Emulation::Chrome134)
        .build()
        .expect("Failed to create client")
});

/// Helper function to add headers to a request
pub trait AppendHeaders {
    fn append_headers(
        self,
        refer: impl AsRef<str>,
        cookies: impl AsRef<str>,
        proxy: Option<Proxy>,
    ) -> Self;
}

impl AppendHeaders for RequestBuilder {
    fn append_headers(
        self,
        refer: impl AsRef<str>,
        cookies: impl AsRef<str>,
        proxy: Option<Proxy>,
    ) -> RequestBuilder {
        let b = self
            .header_append(ORIGIN, ENDPOINT)
            .header_append(REFERER, header_ref(refer))
            .header_append(COOKIE, cookies.as_ref());
        if let Some(proxy) = proxy {
            b.proxy(proxy)
        } else {
            b
        }
    }
}

/// Helper function to get the header reference
fn header_ref<S: AsRef<str>>(ref_path: S) -> String {
    if ref_path.as_ref().is_empty() {
        format!("{}/", ENDPOINT)
    } else {
        format!("{}/chat/{}", ENDPOINT, ref_path.as_ref())
    }
}

/// Upload images to the Claude.ai
pub async fn upload_images(
    imgs: Vec<ImageSource>,
    cookies: String,
    uuid_org: String,
    proxy: Option<Proxy>,
) -> Vec<String> {
    // upload images
    let fut = imgs
        .into_iter()
        .map_while(|img| {
            // check if the image is base64
            if img.type_ != "base64" {
                warn!("Image type is not base64");
                return None;
            }
            // decode the image
            let bytes = BASE64_STANDARD
                .decode(img.data.as_bytes())
                .inspect_err(|e| {
                    warn!("Failed to decode image: {:?}", e);
                })
                .ok()?;
            // choose the file name based on the media type
            let file_name = match img.media_type.as_str() {
                "image/png" => "image.png",
                "image/jpeg" => "image.jpg",
                "image/gif" => "image.gif",
                "image/webp" => "image.webp",
                "application/pdf" => "document.pdf",
                _ => "file",
            };
            // create the part and form
            let part = Part::bytes(bytes).file_name(file_name);
            let form = Form::new().part("file", part);
            let endpoint = format!("https://claude.ai/api/{}/upload", uuid_org);
            Some(
                // send the request into future
                SUPER_CLIENT
                    .post(endpoint)
                    .append_headers("new", cookies.as_str(), proxy.clone())
                    .header_append("anthropic-client-platform", "web_claude_ai")
                    .multipart(form)
                    .send(),
            )
        })
        .collect::<Vec<_>>();

    // get upload responses
    let fut = join_all(fut)
        .await
        .into_iter()
        .map_while(|r| {
            // check if the response is ok
            r.inspect_err(|e| {
                warn!("Failed to upload image: {:?}", e);
            })
            .ok()
        })
        .map(|r| async {
            // get the response json
            // extract the file_uuid
            let json = r
                .json::<Value>()
                .await
                .inspect_err(|e| {
                    warn!("Failed to parse image response: {:?}", e);
                })
                .ok()?;
            Some(json["file_uuid"].as_str()?.to_string())
        })
        .collect::<Vec<_>>();

    // collect the results
    join_all(fut)
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
}
