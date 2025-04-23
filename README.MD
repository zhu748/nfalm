# Clewd<span style="color:#CE422B">R</span>

Complete rewrite of [Clewd修改版](https://github.com/teralomaniac/clewd) in Rust.

## Advantages

- **React Frontend**: ClewdR uses a tiny React frontend for simple configuration. Clewd uses a HTML frontend, which is not very user-friendly.
- **Concurrent**: ClewdR implies a event-driven architecture, which means it can handle multiple requests at the same time. Clewd only handles one request at a time. This is a huge advantage when you are using it as a reverse proxy for SillyTavern.
- **Configurable Proxy**: You can use your own HTTP/HTTPS/SOCKS5 proxy. ClewdR will use the proxy you set in `config.toml` to send requests to Claude. Clewd doesn't support this feature and needs TUN to work with proxy.
- **No Node.js**: ~~Although I'm sure all of you playing SillyTavern have it installed~~
- **No external binaries**: Clewd spawns a `superfetch` command for every request to mimic browser behavior. ClewdR uses native Rust `rquest` library.
- **Support macOS**: Clewd doesn't provide `superfetch` for macOS, so it doesn't work on macOS.
- **Support Android**: Same as above, ClewdR is a native Rust program, so it works on Android.
- **Robust Backend**: Clewd uses home-made backend on top of Node's `Server` Object. ClewdR is built on top of `Axum` and `Tokio`, which are battle-tested libraries for building web servers in Rust.
- **Support Thinking**: Support newest `thinking` feature in SillyTavern for 3.7 Sonnet.
- **Support Image Attachment**: Support image attachment, you can send images to the bot.

## How to start

1. Download binary of your platform from GitHub release page.
2. Run `clewdr` / `clewdr.exe`.
3. Open `http://127.0.1:8484` in your browser to configure the proxy.
4. Fill proxy setting in SillyTavern as Claude Reverse Proxy, **NOT** OpenAI Compatible. Remember to fill the password.
