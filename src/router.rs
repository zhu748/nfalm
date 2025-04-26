use axum::{
    Router,
    http::HeaderMap,
    routing::{delete, get, options, post},
};
use rquest::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
};
use tower_http::services::ServeDir;

use crate::{
    api::{
        api_auth, api_completion, api_delete_cookie, api_get_config, api_get_cookies, api_messages,
        api_post_config, api_submit, api_version,
    },
    config::CLEWDR_CONFIG,
    state::ClientState,
};

/// RouterBuilder for the application
pub struct RouterBuilder {
    state: ClientState,
    inner: Router<ClientState>,
}

impl RouterBuilder {
    /// Creates a blank RouterBuilder instance
    /// Initializes the router with the provided application state
    /// 
    /// # Arguments
    /// * `state` - The application state containing client information
    fn new(state: ClientState) -> Self {
        RouterBuilder {
            state,
            inner: Router::new(),
        }
    }

    /// Creates a new RouterBuilder instance
    /// Sets up routes for API endpoints and static file serving
    ///
    /// # Arguments
    /// * `state` - The application state containing client information
    pub fn new_default(state: ClientState) -> Self {
        Self::new(state)
            .route_v1_endpoints()
            .route_api_endpoints()
            .route_openai_endpoints()
            .setup_static_serving()
    }

    /// Sets up routes for v1 endpoints
    fn route_v1_endpoints(mut self) -> Self {
        self.inner = self
            .inner
            .route("/v1", options(api_options))
            .route("/v1/messages", post(api_messages));
        self
    }

    /// Sets up routes for API endpoints
    fn route_api_endpoints(mut self) -> Self {
        self.inner = self
            .inner
            .route("/api/submit", post(api_submit))
            .route("/api/delete_cookie/{cookie}", delete(api_delete_cookie))
            .route("/api/version", get(api_version))
            .route("/api/get_cookies", get(api_get_cookies))
            .route("/api/auth", get(api_auth))
            .route("/api/config", get(api_get_config).post(api_post_config));
        self
    }

    /// Optionally sets up routes for OpenAI compatible endpoints
    fn route_openai_endpoints(mut self) -> Self {
        if CLEWDR_CONFIG.load().enable_oai {
            self.inner = self
                .inner
                .route("/v1/chat/completions", post(api_completion));
        }
        self
    }

    /// Sets up static file serving
    fn setup_static_serving(mut self) -> Self {
        if cfg!(debug_assertions) {
            self.inner = self.inner.fallback_service(ServeDir::new("static"));
        } else {
            use include_dir::{Dir, include_dir};
            const INCLUDE_STATIC: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");
            self.inner = self
                .inner
                .fallback_service(tower_serve_static::ServeDir::new(&INCLUDE_STATIC));
        }
        self
    }

    /// Returns the configured router
    /// Finalizes the router configuration for use with axum
    pub fn build(self) -> Router {
        self.inner.with_state(self.state)
    }
}

/// Handles CORS preflight requests
/// Sets appropriate CORS headers to allow cross-origin requests
/// Returns a HeaderMap containing the necessary CORS headers
async fn api_options() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        "Authorization, Content-Type".parse().unwrap(),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        "POST, GET, OPTIONS".parse().unwrap(),
    );
    headers
}
