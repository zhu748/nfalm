use axum::http::HeaderValue;
use base64::{Engine, prelude::BASE64_STANDARD};
use futures::{StreamExt, stream};
use rquest::{
    Client, ClientBuilder, IntoUrl, Method, Proxy, RequestBuilder,
    header::{ORIGIN, REFERER},
    multipart::{Form, Part},
};
use rquest_util::Emulation;
use tracing::{debug, error, warn};
use url::Url;

use std::sync::LazyLock;

use crate::{
    api::ApiFormat,
    config::{CLEWDR_CONFIG, CookieStatus, ENDPOINT, Reason},
    error::ClewdrError,
    services::cookie_manager::CookieEventSender,
    types::message::ImageSource,
};

/// Placeholder
static SUPER_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);
/// State of current connection
#[derive(Clone)]
pub struct ClientState {
    pub cookie: Option<CookieStatus>,
    cookie_header_value: HeaderValue,
    pub event_sender: CookieEventSender,
    pub org_uuid: Option<String>,
    pub conv_uuid: Option<String>,
    pub capabilities: Vec<String>,
    pub endpoint: Url,
    pub proxy: Option<Proxy>,
    pub api_format: ApiFormat,
    pub stream: bool,
    pub client: Client,
    pub key: Option<(u64, usize)>,
}

impl ClientState {
    /// Create a new AppState instance
    pub fn new(event_sender: CookieEventSender) -> Self {
        ClientState {
            event_sender,
            cookie: None,
            org_uuid: None,
            conv_uuid: None,
            cookie_header_value: HeaderValue::from_static(""),
            capabilities: Vec::new(),
            endpoint: CLEWDR_CONFIG.load().endpoint(),
            proxy: CLEWDR_CONFIG.load().rquest_proxy.to_owned(),
            api_format: ApiFormat::Claude,
            stream: false,
            client: SUPER_CLIENT.clone(),
            key: None,
        }
    }

    /// Build a request with the current cookie and proxy settings
    pub fn build_request(&self, method: Method, url: impl IntoUrl) -> RequestBuilder {
        // let r = SUPER_CLIENT.cloned();
        self.client
            .set_cookie(&self.endpoint, &self.cookie_header_value);
        let req = self
            .client
            .request(method, url)
            .header_append(ORIGIN, ENDPOINT);
        let req = if let Some(uuid) = self.conv_uuid.to_owned() {
            req.header_append(REFERER, format!("{}/chat/{}", ENDPOINT, uuid))
        } else {
            req.header_append(REFERER, format!("{}/new", ENDPOINT))
        };
        if let Some(proxy) = self.proxy.to_owned() {
            req.proxy(proxy)
        } else {
            req
        }
    }

    /// Checks if the current user has pro capabilities
    /// Returns true if any capability contains "pro", "enterprise", "raven", or "max"
    pub fn is_pro(&self) -> bool {
        self.capabilities.iter().any(|c| {
            c.contains("pro")
                || c.contains("enterprise")
                || c.contains("raven")
                || c.contains("max")
        })
    }

    /// Requests a new cookie from the cookie manager
    /// Updates the internal state with the new cookie and proxy configuration
    pub async fn request_cookie(&mut self) -> Result<(), ClewdrError> {
        let res = self.event_sender.request().await?;
        self.cookie = Some(res.to_owned());
        self.client = ClientBuilder::new()
            .cookie_store(true)
            .emulation(Emulation::Chrome135)
            .build()?;
        self.cookie_header_value = HeaderValue::from_str(res.cookie.to_string().as_str())?;
        // load newest config
        self.proxy = CLEWDR_CONFIG.load().rquest_proxy.to_owned();
        self.endpoint = CLEWDR_CONFIG.load().endpoint();
        Ok(())
    }

    /// Returns the current cookie to the cookie manager
    /// Optionally provides a reason for returning the cookie (e.g., invalid, banned)
    pub async fn return_cookie(&self, reason: Option<Reason>) {
        // return the cookie to the cookie manager
        if let Some(ref cookie) = self.cookie {
            self.event_sender
                .return_cookie(cookie.to_owned(), reason)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to send cookie: {}", e);
                });
        }
    }

    /// Deletes or renames the current chat conversation based on configuration
    /// If preserve_chats is true, the chat is renamed rather than deleted
    pub async fn clean_chat(&self) -> Result<(), ClewdrError> {
        if CLEWDR_CONFIG.load().preserve_chats {
            return Ok(());
        }
        let Some(ref org_uuid) = self.org_uuid else {
            return Ok(());
        };
        let Some(ref conv_uuid) = self.conv_uuid else {
            return Ok(());
        };
        let endpoint = format!(
            "{}/api/organizations/{}/chat_conversations/{}",
            self.endpoint, org_uuid, conv_uuid
        );
        debug!("Deleting chat: {}", conv_uuid);
        let _ = self.build_request(Method::DELETE, endpoint).send().await?;
        Ok(())
    }

    /// Upload images to the Claude.ai
    pub async fn upload_images(&self, imgs: Vec<ImageSource>) -> Vec<String> {
        // upload images
        stream::iter(imgs)
            .filter_map(async |img| {
                // check if the image is base64
                if img.type_ != "base64" {
                    warn!("Image type is not base64");
                    return None;
                }
                // decode the image
                let bytes = BASE64_STANDARD
                    .decode(img.data)
                    .inspect_err(|e| {
                        warn!("Failed to decode image: {}", e);
                    })
                    .ok()?;
                // choose the file name based on the media type
                let file_name = match img.media_type.to_lowercase().as_str() {
                    "image/png" => "image.png",
                    "image/jpeg" => "image.jpg",
                    "image/jpg" => "image.jpg",
                    "image/gif" => "image.gif",
                    "image/webp" => "image.webp",
                    "application/pdf" => "document.pdf",
                    _ => "file",
                };
                // create the part and form
                let part = Part::bytes(bytes).file_name(file_name);
                let form = Form::new().part("file", part);
                let endpoint = format!("{}/api/{}/upload", self.endpoint, self.org_uuid.as_ref()?);
                // send the request into future
                let res = self
                    .build_request(Method::POST, endpoint)
                    .multipart(form)
                    .send()
                    .await
                    .inspect_err(|e| {
                        warn!("Failed to upload image: {}", e);
                    })
                    .ok()?;
                #[derive(serde::Deserialize)]
                struct UploadResponse {
                    file_uuid: String,
                }
                // get the response json
                let json = res
                    .json::<UploadResponse>()
                    .await
                    .inspect_err(|e| {
                        warn!("Failed to parse image response: {}", e);
                    })
                    .ok()?;
                // extract the file_uuid
                Some(json.file_uuid)
            })
            .collect::<Vec<_>>()
            .await
    }
}
