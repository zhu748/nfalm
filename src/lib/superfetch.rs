use rquest::{Client, Request, RequestBuilder, header::HeaderMap};
use rquest_util::Emulation;
use serde_json::Value;

// pub struct FetchConfig {
//     pub request: Request,
// }

// pub async fn superfetch(config: FetchConfig) {
//     // Build a client
//     let client = Client::builder()
//         .emulation(Emulation::Chrome134)
//         .build()
//         .unwrap();

//     // Use the API you're already familiar with
//     let resp = client.execute(config.request).await.unwrap();
//     println!("{}", resp.text().await.unwrap());
// }

#[cfg(test)]
mod tests {

    use super::*;
    const TEST_COOKIE: &str = "sessionKey=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";
    #[tokio::test]
    async fn test_bootstrap() {
        let client = Client::builder()
            .emulation(Emulation::Chrome134)
            .build()
            .unwrap();

        let resp = client
            .get("https://api.claude.ai/api/bootstrap")
            .header_append("Cookie", TEST_COOKIE)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );
        let json: Value = resp.json().await.unwrap();
        println!("{}", json["account"].to_string());
    }
}
