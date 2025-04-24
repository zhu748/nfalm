# 使用 Node.js 镜像作为前端构建环境
FROM node:20-slim AS frontend-builder
WORKDIR /usr/src/app/frontend
# 安装pnpm
RUN npm install -g pnpm
# 复制前端源码
COPY frontend/ .
# 安装依赖并构建前端
RUN pnpm install && pnpm run build
# 注意：前端构建结果会输出到 ../static 目录中

# 使用 rust:1 镜像作为后端构建环境
FROM rust:1 AS backend-builder
# 安装构建依赖
RUN apt-get update && apt-get install -y \
    cmake \
    clang \
    pkg-config \
    libssl-dev \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/clewdr
# 复制项目源码
COPY . .
# 复制前端构建产物到static目录
COPY --from=frontend-builder /usr/src/app/static ./static
# 构建后端（release 模式）
ENV RUSTFLAGS=-Awarnings
RUN RUST_BACKTRACE=1 cargo build --release --features no_fs

# 使用更小的基础镜像
FROM debian:bookworm-slim
# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
# 创建应用目录结构
RUN mkdir -p /app/log /app/static
WORKDIR /app
# 从后端构建阶段复制编译好的二进制文件
COPY --from=backend-builder /usr/src/clewdr/target/release/clewdr .
# 从后端构建阶段复制静态文件
COPY --from=backend-builder /usr/src/clewdr/static ./static

# 设置卷挂载
VOLUME ["/app/log"]

# 配置环境变量
ENV CLEWDR_IP=0.0.0.0
ENV CLEWDR_PORT=8484
ENV CLEWDR_CHECK_UPDATE=0
ENV CLEWDR_AUTO_UPDATE=0

# 暴露端口
EXPOSE 8484

# 启动命令
CMD ["./clewdr"]
