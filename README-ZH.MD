# Clewd<span style="color:#CE422B">R</span>

[English](./README.MD) | 简体中文

基于 Rust 对[Clewd 修改版](https://github.com/teralomaniac/clewd)的完全重写。

## 社区教程

[Clewdr 部署到 huggingface space](./wiki/hf-space.md)

## 优势

- **高性能**：ClewdR 使用 Rust 进行高性能处理，流处理速度数倍于原版 Clewd。
- **并发**：基于事件驱动架构，可同时处理多请求。原版仅支持单请求处理。
- **React 前端**：采用轻量级 React 前端实现热重载配置。原版 Clewd 只能使用配置文件、环境变量或者 API query配置。
- **可配置代理**：支持自定义 HTTP/HTTPS/SOCKS5 代理，原版不支持此功能需依赖 TUN。
- **无需 Node.js**：~~虽然你们玩 SillyTavern 的肯定都装了~~
- **无外部依赖**：原版每个请求都需调用`superfetch`模拟浏览器行为，本作使用 Rust 原生`rquest`库实现。
  - **支持 macOS**：原版因缺少 macOS 版`superfetch`无法运行。
  - **支持 Android**：基于 Rust 原生编译，完美兼容 Android 系统。
- **健壮后端**：原版采用自研 Node.js 后端，本作基于久经考验的 Rust 生态组件`Axum`和`Tokio`构建。
- **思维支持**：支持 SillyTavern 最新 3.7 Sonnet 版本的`请求思维链`功能。
- **图片附件**：图片附件支持，可直接发送图片给 AI。

## 使用指南

1. 从 GitHub 发布页下载对应平台二进制文件
2. 运行`clewdr`或`clewdr.exe`
3. 浏览器访问`http://127.0.1:8484`配置代理
4. 在 SillyTavern 中设置为 Claude 反向代理（**非**OpenAI 兼容模式），务必填写密码
