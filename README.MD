# Clewd<span style="color:#CE422B">R</span>

English | [简体中文](./README-ZH.MD)

**ClewdR** is a high-performance, feature-rich Rust implementation of Claude reverse proxy, completely rewritten from the ground up to overcome the limitations of the original [Clewd修改版](https://github.com/teralomaniac/clewd). Designed for speed, reliability, and ease of use, ClewdR offers a seamless way to interact with Claude AI models while bringing significant improvements to the user experience.

## Key Features

| Feature | ClewdR | Original Clewd |
|---------|--------|----------------|
| **Performance** | Fast Rust processing | Slower processing |
| **Concurrent** | Multiple concurrent requests | Single request only |
| **Configuration** | React UI with hot-reload | Config files/env vars only |
| **Proxy Support** | HTTP/HTTPS/SOCKS5 proxies | No proxy support (needs TUN) |
| **Dependencies** | No Node.js required | Requires Node.js |
| **HTTP Client** | Native Rust `rquest` | Requires `superfetch` |
| **Platform Support** | macOS and Android compatible | Not compatible because of lack `superfetch` |
| **Backend** | `Axum` and `Tokio` | Custom Node.js backend |
| **Claude 3.7** | Supports `thinking` feature | No support |
| **Images** | Supports image attachments | No support |

## How to Start

1. Download binary for your platform from [GitHub releases](https://github.com/xerxes-2/clewdr/releases).
2. Run `clewdr` / `clewdr.exe`.
3. Open `http://127.0.0.1:8484` in your browser to configure the proxy.
4. In SillyTavern: Set as Claude Reverse Proxy, **NOT** OpenAI Compatible. Remember to fill password.

## System Requirements

- Windows 8+, macOS 10.12+, Linux, Android
- Prebuilt Linux binaries require glibc 2.38 or later
  - You can use `musl`-based binaries for older systems
- No additional runtime dependencies required

## Configuration Options

Access the web UI at `http://127.0.0.1:8484` to configure:

- Proxy settings
- Authentication options
- Claude API parameters
- Request handling preferences

## Troubleshooting

- **Connection Issues**: Verify network connectivity and proxy settings
- **Authentication Errors**: Ensure correct password is configured

## Community Resources

- [Deploy ClewdR to HuggingFace Space (Chinese)](./wiki/hf-space.md)

## Contributing

Contributions welcome! Feel free to submit issues or pull requests on GitHub.
