# v0.11.20

## Highlights

- Claude tool-result content now accepts structured array payloads, keeping MCP clients such as OpenHands compatible with the proxy.

## Improvements

- Normalized endpoint constants and switched to `Url::join` when building Claude Code, Claude Web, Gemini, and OAuth URLs, avoiding double-slash mistakes when overriding API hosts.
- Refined Claude message sanitization so blank assistant turns are dropped without discarding legitimate content blocks.

## Bug Fixes

- Fixed Claude Code chat and token-count requests failing when the configured endpoint already included a trailing slash.

## Dependency Updates

- Bumped `zip` to 6.x and `enable-ansi-support` to 0.3, refreshing the lockfile to the latest compatible releases.
