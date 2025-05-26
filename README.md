# Clewd<span style="color:#CE422B">R</span>

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Xerxes-2/clewdr)
[![GitHub Release](https://img.shields.io/github/v/release/Xerxes-2/clewdr?style=flat-square)](https://github.com/Xerxes-2/clewdr/releases/latest)

[English](./README.md) | [简体中文](./README_zh.md)

## High-Performance LLM Proxy

Specifically built for Claude (Claude.ai) and Gemini (Google AI Studio, Google Vertex AI)

## Core Advantages

### Full-Featured Frontend

- Integrated React frontend providing a complete functional experience

### Efficient Architecture

- Occupies one-tenth the resources compared to script language implementations, with ten times the performance, easily handling thousands of requests per second
- Event-driven design, decoupled logic, supports hot reloading and multiple configuration methods
- High-performance response caching supported by Moka technology
- Multi-threaded asynchronous processing based on Tokio and Axum
- Fingerprint-level Chrome simulation Rquest HTTP client

### Intelligent Cookie Management

- Automatic classification and management of account status
- Fine-grained polling mechanism to maximize resource utilization

### Full Platform Compatibility

- Rust static compilation, single binary deployment, no environment dependencies needed
- Native support for macOS/Android and other platforms
- Extremely low memory usage (only single-digit MB)
- No need for virtual machines or complex dependencies

### Enhanced Features

- Built-in proxy server support (no TUN required)
- Concurrent cache request handling
- Gemini additional support:
  - Google AI Studio and Google Vertex AI
  - OpenAI compatible mode / Gemini format
  - Painless HTTP Keep-Alive support
- Claude additional support:
  - OpenAI compatible mode / Claude format
  - Extend Thinking
  - Stop sequences implemented on the proxy side
  - Image attachment uploads
  - Web search
  - Claude Max

## Quick Start

1. Download the program package for your platform ([Latest Version](https://github.com/Xerxes-2/clewdr/releases/latest))
2. The password will be automatically generated on the first run. Access the default frontend address <http://127.0.0.1:8484>, and log in using the Web Admin Password displayed in the console.
   - If you need to change the password, you can set a new one in the frontend interface.
   - If you forget the password, you can delete the `clewdr.toml` file to regenerate it.
   - Note: If deploying with Docker, the password will be generated and displayed in the logs when the container starts.
3. Configure the proxy address and other parameters in the frontend interface, add Cookies and Keys.
4. Third-party application configuration:
    1. ClewdR will print the access addresses for each API in the console when it starts.
    2. Choose the API format you want (Claude or Gemini or OpenAI compatible).
    3. Set the corresponding proxy address in applications like SillyTavern, and fill in the API Password displayed in the console as the proxy password.
5. Enjoy the high-performance LLM proxy service!

## Community Resources

**Github Aggregated Wiki**: <https://github.com/Xerxes-2/clewdr/wiki>
