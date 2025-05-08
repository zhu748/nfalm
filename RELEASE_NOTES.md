# v0.9.2

## Major Bug Fixes, Recommand to Upgrade immediately

- Fix many false use of `map_while` to `filter_map`, that can lead to ignore some messages.

## New Features

- **Google Vertex AI**: Added support for Google Vertex AI, config in frontend.
- **Gemini Non-stream**: Support connection keep-alive for Gemini non-streaming, no false stream needed.
