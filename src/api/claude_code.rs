use crate::{
    claude_code_state::ClaudeCodeState,
    error::ClewdrError,
    middleware::claude::{ClaudeContext, ClaudePreprocess},
    utils::{enabled, print_out_json},
};
use axum::{Extension, extract::State, response::Response};
use colored::Colorize;
use tracing::info;

pub async fn api_claude_code(
    State(mut state): State<ClaudeCodeState>,
    ClaudePreprocess(p, f): ClaudePreprocess,
) -> (Extension<ClaudeContext>, Result<Response, ClewdrError>) {
    print_out_json(&p, "client_req.json");
    info!(
        "[{}] stream: {}, msgs: {}, model: {}, think: {}",
        "CLAUDE CODE".red(),
        enabled(p.stream.unwrap_or_default()),
        p.messages.len().to_string().green(),
        p.model.green(),
        enabled(p.thinking.is_some()),
    );
    (Extension(f), state.try_chat(p).await)
}
