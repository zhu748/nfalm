use crate::{
    claude_code_state::ClaudeCodeState,
    config::CLEWDR_CONFIG,
    error::ClewdrError,
    middleware::claude::{ClaudeContext, ClaudePreprocess},
    types::claude_message::ContentBlock,
    utils::{enabled, print_out_json},
};
use axum::{Extension, extract::State, response::Response};
use colored::Colorize;
use serde_json::Value;
use tracing::info;

pub async fn api_claude_code(
    State(mut state): State<ClaudeCodeState>,
    ClaudePreprocess(mut p, f): ClaudePreprocess,
) -> (Extension<ClaudeContext>, Result<Response, ClewdrError>) {
    print_out_json(&p, "client_req.json");
    p.stop = p.stop_sequences.clone();
    info!(
        "[{}] stream: {}, msgs: {}, model: {}, think: {}",
        "CLAUDE CODE".red(),
        enabled(p.stream.unwrap_or_default()),
        p.messages.len().to_string().green(),
        p.model.green(),
        enabled(p.thinking.is_some()),
    );
    let prelude = ContentBlock::Text {
        text: CLEWDR_CONFIG
            .load()
            .custom_system
            .clone()
            .unwrap_or_else(|| "You are Claude Code, Anthropic's official CLI for Claude.".into()),
    };
    let mut system = vec![serde_json::to_value(prelude).unwrap()];
    match p.system {
        Some(Value::String(ref s)) => {
            let text_content = ContentBlock::Text { text: s.clone() };
            system.push(serde_json::to_value(text_content).unwrap());
        }
        Some(Value::Array(ref a)) => system.extend(a.clone()),
        _ => {}
    }
    p.system = Some(Value::Array(system));
    (Extension(f), state.try_chat(p).await)
}
