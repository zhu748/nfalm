# Clewd<span style="color:#CE422B">R</span>

English | [简体中文](./README_zh.md)

**ClewdR** is a high-performance, feature-rich Rust implementation of Claude reverse proxy, completely rewritten from the ground up to overcome the limitations of the original [Clewd修改版](https://github.com/teralomaniac/clewd). Designed for speed, reliability, and ease of use, ClewdR offers a seamless way to interact with Claude AI models while bringing significant improvements to the user experience.

## Key Features

| Feature | ClewdR | Original Clewd |
|---------|--------|----------------|
| **Performance** | 3x Faster stream | Slow |
| **Caching Responses** |  Supported | N/A |
| **Memory Usage** | < 10MB | High |
| **Concurrency** | Multithreaded concurrent requests | Single-threaded single request |
| **Deployment** | Docker / Single Binary | Complex setup |
| **Configuration** | React UI / File / Env | File / Env |
| **Hot Reload** | Supported | N/A |
| **Cookie Management** | Automatic | Limited |
| **Proxy Support** | HTTP/HTTPS/SOCKS5 | N/A (needs TUN) |
| **Dependencies** | N/A | Requires Node.js |
| **HTTP Client** | Internal Rust `rquest` | External `superfetch` binary |
| **Platform Support** | macOS and Android native | Not native, lack of `superfetch` |
| **Backend** | `Axum` and `Tokio` | Custom Node.js backend |
| **Extend Thinking** | Supported | N/A |
| **Images** | Supported | N/A |

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

- [Getting Tokens Using Vertex](./wiki/vertex.md)
- [Deploy ClewdR to HuggingFace Space (Chinese)](./wiki/hf-space.md)

## Contributing

Contributions welcome! Feel free to submit issues or pull requests on GitHub.
