# 使用 Node.js 镜像作为前端构建环境
FROM node:lts-slim AS frontend-builder
WORKDIR /usr/src/app/frontend
# 安装pnpm
RUN npm install -g pnpm
# 复制前端源码
COPY frontend/ .
# 安装依赖并构建前端
RUN pnpm install && pnpm run build
# 注意：前端构建结果会输出到 ../static 目录中

FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS backend-builder 
# Install musl target and required dependencies
RUN rustup target add x86_64-unknown-linux-musl
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY . .
ENV RUSTFLAGS="-Awarnings"
COPY --from=frontend-builder /usr/src/app/static ./static
RUN cargo build --release --target x86_64-unknown-linux-musl --bin clewdr --features no_fs

# 使用 distroless 静态镜像
FROM gcr.io/distroless/static-debian12
WORKDIR /app
# 从后端构建阶段复制编译好的二进制文件
COPY --from=backend-builder /app/target/x86_64-unknown-linux-musl/release/clewdr .

# 配置环境变量
ENV CLEWDR_IP=0.0.0.0
ENV CLEWDR_PORT=8484
ENV CLEWDR_CHECK_UPDATE=FALSE
ENV CLEWDR_AUTO_UPDATE=FALSE
ENV CLEWDR_TOKIO_CONSOLE=FALSE

# 暴露端口
EXPOSE 8484

# 启动命令
CMD ["./clewdr"]
