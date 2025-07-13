# Clewd<span style="color:#CE422B">R</span>

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Xerxes-2/clewdr)
[![GitHub Release](https://img.shields.io/github/v/release/Xerxes-2/clewdr?style=flat-square)](https://github.com/Xerxes-2/clewdr/releases/latest)

## 高性能 LLM 代理

专为 Claude (Claude.ai) 和 Gemini (Google AI Studio, Google Vertex AI) 打造

## 核心优势

### 全功能前端

- 集成式 React 前端，提供完整功能体验

### 高效架构

- 相比脚本语言实现占用十分之一，性能十倍，轻松每秒上千请求
- 事件驱动设计，逻辑解耦，支持热重载和多种配置方式
- Moka 技术支持的高性能回复缓存
- 基于 Tokio 和 Axum 的多线程异步处理
- 指纹级模拟 Chrome 的 Rquest HTTP 客户端

### 智能 Cookie 管理

- 自动分类管理账号状态
- 精细化轮询机制，最大化资源利用

### 全平台兼容

- Rust 静态编译，单二进制部署，不需要环境依赖
- 原生支持 macOS/Android 等多平台
- 极低内存占用（仅个位数 MB）
- 无需虚拟机或复杂依赖

### 增强功能

- 内置代理服务器支持（无需 TUN）
- 并发缓存请求处理
- Gemini 额外支持：
  - Google AI Studio 和 Google Vertex AI
  - OpenAI 兼容模式 / Gemini 格式
  - 无痛 Http Keep-Alive 支持
- Claude 额外支持：
  - Claude Code
  - 系统提示缓存
  - OpenAI 兼容模式 / Claude 格式
  - Extend Thinking 扩展思考
  - 代理端实现停止序列
  - 图片附件上传
  - 网页搜索
  - Claude Max

## 快速上手

1. 下载对应平台的程序包（[最新版本](https://github.com/Xerxes-2/clewdr/releases/latest)）
2. 首次运行将自动生成密码，访问默认前端地址 <http://127.0.0.1:8484>，使用控制台显示的 Web Admin Password 进行登录
   - 如果需要修改密码，可以在前端界面中设置新的密码
   - 如果忘记密码，可以删除 `clewdr.toml` 文件重新生成
   - 注意：如果使用 Docker 部署，密码会在容器启动时生成并显示在日志中
3. 在前端界面中配置代理地址和其他参数，添加 Cookie 和 Key
4. 第三方应用配置：
    1. ClewdR 启动时会在控制台打印各个 API 的访问地址
    2. 选择你想要的 API 格式（Claude 或 Gemini 或 OpenAI 兼容）
    3. 在 SillyTavern 等应用中设置为相应的代理地址，代理密码填写控制台显示的 API Password
5. 享受高性能 LLM 代理服务！

## 社区资源

**Github 聚合 Wiki**：<https://github.com/Xerxes-2/clewdr/wiki>

## 致谢

- [Clewd 修改版](https://github.com/teralomaniac/clewd) - 原始 Clewd 的修改版，提供了许多灵感和基础功能
- [Clove](https://github.com/mirrorange/clove) - 提供了 Claude Code 的支持逻辑
