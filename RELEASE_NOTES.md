# v0.10.7

## Features

- **OAuth2 Integration**: Added OAuth2 authorization flow to replace PKCE implementation for improved authentication
- **Enhanced Caching**: Added system prompt hash to ClaudeCodeState and improved cookie management for better caching performance

## Improvements

- **Configuration Optimization**: Enhanced configuration retrieval with improved caching and hash implementation
- **Error Handling**: Simplified error handling for memberships and organization retrieval in get_organization method
- **Code Quality**: Refactored OAuth client initialization to remove unnecessary conversions and improve readability

## Dependencies

- **HTTP Client**: Replaced rquest with wreq for HTTP client functionality
- **Updated Dependencies**: Updated async-compression, toml, toml_parser, toml_writer, and winnow to latest versions

## Files Changed

- 22 files modified with 365 insertions and 276 deletions
- Major changes to authentication flow, caching system, and HTTP client implementation
