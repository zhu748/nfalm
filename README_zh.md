# Clewd<span style="color:#CE422B">R</span>

[English](./README.md) | 简体中文

**ClewdR** 是一个高性能、功能丰富的 Claude/Gemini 逆向代理实现，使用 Rust 语言完全重写，旨在克服原版 [Clewd 修改版](https://github.com/teralomaniac/clewd) 的局限性。ClewdR 设计注重速度、可靠性和易用性，为用户提供与 Claude AI 模型交互的无缝体验，同时显著改善用户体验。

## 核心功能

| 功能 | ClewdR | 原版 Clewd |
|------|--------|------------|
| **性能** | 流式传输速度提升3倍 | 处理速度较慢 |
| **缓存响应** | 支持 | 不适用 |
| **内存占用** | < 10MB | 较高 |
| **并发** | 多线程并发请求 | 单线程单请求 |
| **部署** | Docker / 单一二进制文件 | 复杂设置 |
| **配置** | React UI / 文件 / 环境变量 | 文件 / 环境变量 |
| **热重载** | 支持 | 不适用 |
| **Cookie 管理** | 自动 | 有限 |
| **代理支持** | HTTP/HTTPS/SOCKS5 | 不适用 (需要 TUN) |
| **依赖** | 无 | 需要 Node.js |
| **HTTP 客户端** | 内置 Rust `rquest` | 外部 `superfetch` 二进制文件 |
| **平台支持** | macOS 和 Android 原生支持 | 非原生支持，缺少 `superfetch` |
| **后端** | `Axum` 和 `Tokio` | 自制 Node.js 后端 |
| **思维链扩展** | 支持 | 不适用 |
| **图片** | 支持 | 不适用 |
| **Gemini 支持** | Google AI Studio 和 Vertex AI | 不适用 |
| **多 API 格式** | Claude, Gemini, OpenAI | 仅 OpenAI |

## 使用指南

1. 从 [GitHub 发布页](https://github.com/xerxes-2/clewdr/releases) 下载对应平台二进制文件
2. 运行`clewdr`或`clewdr.exe`
3. 浏览器访问`http://127.0.0.1:8484`配置代理
4. 在 SillyTavern 中设置为 Claude 反向代理（**非**OpenAI 兼容模式），务必填写密码
   - 完美支持 SillyTavern 非流式模式（无需假流式传输）
   - 同时支持 Gemini 和 Claude 模型

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
- Gemini API 选项（Google AI Studio 和 Vertex AI）
- 请求处理偏好设置

## 故障排除

- **连接问题**：检查网络连接和代理设置
- **认证错误**：确保配置了正确的密码

## 社区资源

- [Vertex用token获取教程](https://github.com/wyeeeee/hajimi/blob/main/wiki/vertex.md)
- [Linux一键脚本](https://github.com/rzline/st-cr-ins.sh)
- [ClewdR 部署到 HuggingFace Space](./wiki/hf-space.md)

## 贡献

欢迎贡献！请随时在 GitHub 上提交问题或拉取请求。
