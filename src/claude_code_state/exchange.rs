use std::{collections::HashMap, str::FromStr};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use http::Method;
use pkce_std::Code;
use serde_json::{Value, json};
use snafu::ResultExt;
use url::Url;

use crate::{
    claude_code_state::ClaudeCodeState,
    config::{CC_REDIRECT_URI, CC_TOKEN_URL, CLEWDR_CONFIG, TokenInfo, TokenInfoRaw},
    error::{CheckClaudeErr, ClewdrError, RquestSnafu, UrlSnafu},
};

pub struct ExchangeResult {
    code: String,
    state: Option<String>,
    verifier: String,
}

impl ClaudeCodeState {
    pub async fn exchange_code(&self, org_uuid: &str) -> Result<ExchangeResult, ClewdrError> {
        let authorize_url = |org_uuid: &str| {
            format!(
                "{}/v1/oauth/{}/authorize",
                CLEWDR_CONFIG.load().endpoint(),
                org_uuid
            )
        };
        let cc_client_id = CLEWDR_CONFIG.load().cc_client_id();
        let state = BASE64_URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
        let code = Code::generate_default();
        let (verifier, challenge) = code.into_pair();
        let code_payload = json!({
            "response_type": "code",
            "client_id": cc_client_id,
            "organization_uuid": org_uuid,
            "redirect_uri": CC_REDIRECT_URI,
            "scope": "user:profile user:inference",
            "state": state,
            "code_challenge": challenge.to_string(),
            "code_challenge_method": "S256",
        });
        let resp = self
            .build_request(Method::POST, authorize_url(org_uuid))
            .json(&code_payload)
            .send()
            .await
            .context(RquestSnafu {
                msg: "Failed to exchange code",
            })?
            .check_claude()
            .await?;

        let json = resp.json::<Value>().await.context(RquestSnafu {
            msg: "Failed to parse exchange code response",
        })?;
        let redirect_uri = json["redirect_uri"]
            .as_str()
            .expect("Expected redirect_uri in response");
        let redirect_url = Url::from_str(redirect_uri).context(UrlSnafu {
            url: redirect_uri.to_string(),
        })?;
        // get code from redirect URL
        let query = redirect_url.query_pairs().collect::<HashMap<_, _>>();
        let code = query.get("code").ok_or(ClewdrError::UnexpectedNone {
            msg: "No code found in redirect URL",
        })?;
        let state = query.get("state");

        Ok(ExchangeResult {
            code: code.to_string(),
            state: state.map(|s| s.to_string()),
            verifier: verifier.to_string(),
        })
    }

    pub async fn exchange_token(&mut self, code_res: ExchangeResult) -> Result<(), ClewdrError> {
        let client_id = CLEWDR_CONFIG.load().cc_client_id();
        let exchange_payload = json!({
            "code": code_res.code,
            "grant_type": "authorization_code",
            "client_id": client_id,
            "redirect_uri": CC_REDIRECT_URI,
            "code_verifier": code_res.verifier,
            "state": code_res.state,
        });
        let token_response = self
            .build_request(Method::POST, CC_TOKEN_URL)
            .json(&exchange_payload)
            .send()
            .await
            .context(RquestSnafu {
                msg: "Failed to exchange token",
            })?
            .check_claude()
            .await?;
        let token_info = token_response
            .json::<TokenInfoRaw>()
            .await
            .context(RquestSnafu {
                msg: "Failed to parse token info",
            })?;
        if let Some(cookie) = self.cookie.as_mut() {
            cookie.token = Some(TokenInfo::new(token_info));
        } else {
            return Err(ClewdrError::UnexpectedNone {
                msg: "No cookie found to update with token info",
            });
        }
        Ok(())
    }

    pub async fn refresh_token(&mut self) -> Result<(), ClewdrError> {
        let Some(cookie) = self.cookie.to_owned() else {
            return Err(ClewdrError::UnexpectedNone {
                msg: "No cookie found to refresh token",
            });
        };
        let Some(token) = cookie.token else {
            return Err(ClewdrError::UnexpectedNone {
                msg: "No token found in cookie to refresh",
            });
        };
        if !token.is_expired() {
            return Ok(());
        }
        let client_id = CLEWDR_CONFIG.load().cc_client_id();
        let refresh_payload = json!({
            "grant_type": "refresh_token",
            "client_id": client_id,
            "refresh_token": token.refresh_token,
        });
        let token_response = self
            .build_request(Method::POST, CC_TOKEN_URL)
            .json(&refresh_payload)
            .send()
            .await
            .context(RquestSnafu {
                msg: "Failed to refresh token",
            })?
            .check_claude()
            .await?;
        let token_info = token_response
            .json::<TokenInfoRaw>()
            .await
            .context(RquestSnafu {
                msg: "Failed to parse refreshed token info",
            })?;
        if let Some(cookie) = self.cookie.as_mut() {
            cookie.token = Some(TokenInfo::new(token_info));
        } else {
            return Err(ClewdrError::UnexpectedNone {
                msg: "No cookie found to update with refreshed token info",
            });
        }
        Ok(())
    }
}
