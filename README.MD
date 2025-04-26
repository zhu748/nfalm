# Clewd<span style="color:#CE422B">R</span>

English | [简体中文](./README-ZH.MD)

Complete rewrite of [Clewd修改版](https://github.com/teralomaniac/clewd) in Rust.

## Community Tutorial

[Deploy Clewdr to HuggingFace Space(Chinese)](./wiki/hf-space.md)

## Advantages

- **High Performance**: ClewdR uses Rust for high-performance processing, with stream processing speed several times that of the original Clewd.
- **Concurrent**: ClewdR uses an event-driven architecture that can handle multiple requests simultaneously. The original version only supports single request processing.
- **React Frontend**: ClewdR uses a lightweight React frontend for simple configuration. The original Clewd only supports configuration via config files, environment variables, or API query parameters.
- **Configurable Proxy**: Supports custom HTTP/HTTPS/SOCKS5 proxies. The original version doesn't support this feature and requires TUN.
- **No Node.js**: ~~Although I'm sure all of you playing SillyTavern have it installed~~
- **No External Dependencies**: The original version requires calling `superfetch` to simulate browser behavior for each request. This version uses Rust's native `rquest` library.
  - **Support macOS**: The original version cannot run on macOS due to lack of macOS version of `superfetch`.
  - **Support Android**: Built with native Rust compilation, perfectly compatible with Android systems.
- **Robust Backend**: The original version uses a self-developed Node.js backend. This version is built on well-tested Rust ecosystem components `Axum` and `Tokio`.
- **Thinking Support**: Supports the newest `thinking` feature in SillyTavern for Claude 3.7 Sonnet.
- **Image Attachments**: Supports image attachments, allowing you to send images directly to AI.

## How to start

1. Download binary of your platform from GitHub release page.
2. Run `clewdr` / `clewdr.exe`.
3. Open `http://127.0.1:8484` in your browser to configure the proxy.
4. Fill proxy setting in SillyTavern as Claude Reverse Proxy, **NOT** OpenAI Compatible. Remember to fill the password.
