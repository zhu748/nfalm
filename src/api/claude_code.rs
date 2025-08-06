use std::time::Instant;

use axum::{Extension, extract::State, response::Response};
use colored::Colorize;
use tracing::info;

use crate::{
    claude_code_state::ClaudeCodeState,
    error::ClewdrError,
    middleware::claude::{ClaudeApiFormat, ClaudeCodePreprocess, ClaudeContext},
    utils::{enabled, print_out_json},
};

pub async fn api_claude_code(
    State(mut state): State<ClaudeCodeState>,
    ClaudeCodePreprocess(p, f): ClaudeCodePreprocess,
) -> Result<(Extension<ClaudeContext>, Response), ClewdrError> {
    state.system_prompt_hash = f.system_prompt_hash();
    state.stream = p.stream.unwrap_or_default();
    state.api_format = f.api_format();
    state.usage = f.usage().to_owned();
    print_out_json(&p, "client_req.json");
    let format_display = match f.api_format() {
        ClaudeApiFormat::Claude => ClaudeApiFormat::Claude.to_string().green(),
        ClaudeApiFormat::OpenAI => ClaudeApiFormat::OpenAI.to_string().yellow(),
    };
    info!(
        "[REQ] stream: {}, msgs: {}, model: {}, think: {}, format: {}",
        enabled(state.stream),
        p.messages.len().to_string().green(),
        p.model.green(),
        enabled(p.thinking.is_some()),
        format_display
    );
    let stopwatch = Instant::now();
    let res = state.try_chat(p).await;

    let elapsed = stopwatch.elapsed();
    info!(
        "[FIN] elapsed: {}s",
        format!("{}", elapsed.as_secs_f32()).green()
    );

    res.map(|r| (Extension(f), r))
}
