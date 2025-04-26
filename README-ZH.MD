# Clewd<span style="color:#CE422B">R</span>

[English](./README.MD) | 简体中文

**ClewdR** 是一个高性能、功能丰富的 Claude 逆向代理实现，使用 Rust 语言完全重写，旨在克服原版 [Clewd 修改版](https://github.com/teralomaniac/clewd) 的局限性。ClewdR 设计注重速度、可靠性和易用性，为用户提供与 Claude AI 模型交互的无缝体验，同时显著改善用户体验。

## 核心功能

| 功能 | ClewdR | 原版 Clewd |
|------|--------|------------|
| **性能** | 使用 Rust 进行高性能处理 | 处理速度较慢 |
| **并发** | 基于事件驱动架构，可并发处理多请求 | 仅支持单请求处理 |
| **配置界面** | 采用轻量级 React 前端实现热重载配置 | 只能使用配置文件、环境变量或者 API query配置 |
| **代理支持** | 支持自定义 HTTP/HTTPS/SOCKS5 代理 | 不支持此功能需依赖 TUN |
| **Node.js 依赖** | 无需 Node.js | 需要 Node.js |
| **外部依赖** | 使用 Rust 原生`rquest`库实现 | 每个请求都需调用`superfetch`模拟浏览器行为 |
| **平台支持** | 完全支持 macOS 和 Android | 因缺少原生`superfetch`无法直接运行 |
| **后端** | 基于久经考验的 Rust 生态组件`Axum`和`Tokio`构建 | 采用自研 Node.js 后端 |
| **Claude 3.7 功能** | 支持 SillyTavern 最新版本的`请求思维链`功能 | 不支持 |
| **图片附件** | 支持直接发送图片给 AI | 不支持 |

## 使用指南

1. 从 [GitHub 发布页](https://github.com/xerxes-2/clewdr/releases) 下载对应平台二进制文件
2. 运行`clewdr`或`clewdr.exe`
3. 浏览器访问`http://127.0.0.1:8484`配置代理
4. 在 SillyTavern 中设置为 Claude 反向代理（**非**OpenAI 兼容模式），务必填写密码

## 系统要求

- Windows 8+, macOS 10.12+, Linux, Android
- Linux 预构建二进制文件需要 glibc 2.38 或更新版本
  - 较旧系统可使用基于 musl 的二进制文件
- 无需额外运行时依赖

## 配置选项

访问网页界面 `http://127.0.0.1:8484` 进行配置：

- 代理设置
- 认证选项
- Claude API 参数
- 请求处理偏好设置

## 故障排除

- **连接问题**：检查网络连接和代理设置
- **认证错误**：确保配置了正确的密码

## 社区资源

- [Clewdr 部署到 HuggingFace Space](./wiki/hf-space.md)

## 贡献

欢迎贡献！请随时在 GitHub 上提交问题或拉取请求。
