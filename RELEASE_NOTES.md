# v0.11.18

## Bug Fixes

- Claude API: trim whitespace-only assistant messages before dispatching so Anthropic no longer returns 400 for streams that include blank turns.
