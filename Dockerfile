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

FROM lukemathwalker/cargo-chef:latest-rust-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS backend-builder
ARG TARGETPLATFORM
WORKDIR /app
# Install musl target and required dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    musl-tools \
    musl-dev \
    cmake \
    clang \
    libclang-dev \
    perl \
    pkg-config \
    mold \
    g++-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl && \
    rustup target add aarch64-unknown-linux-musl

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN <<EOF
set -e
case ${TARGETPLATFORM} in \
    "linux/amd64") \
        RUST_TARGET="x86_64-unknown-linux-musl"
        export CXX="x86_64-linux-gnu-g++"
        ;; \
    "linux/arm64") \
        RUST_TARGET="aarch64-unknown-linux-musl"
        export CXX="aarch64-linux-gnu-g++"
        ;; \
    *) echo "Unsupported architecture: ${TARGETPLATFORM}" >&2; exit 1 ;; \
esac
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml <<EOM
[target.${RUST_TARGET}]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
EOM
cargo chef cook --release --target ${RUST_TARGET} --recipe-path recipe.json
EOF

# Build application
COPY . .
ENV RUSTFLAGS="-Awarnings"
COPY --from=frontend-builder /usr/src/app/static ./static
RUN <<EOF
set -e
case ${TARGETPLATFORM} in \
    "linux/amd64") \
        RUST_TARGET="x86_64-unknown-linux-musl"
        ;; \
    "linux/arm64") \
        RUST_TARGET="aarch64-unknown-linux-musl"
        ;; \
    *) echo "Unsupported architecture: ${TARGETPLATFORM}" >&2; exit 1 ;; \
esac
cargo build --release --target ${RUST_TARGET} --bin clewdr
cp /app/target/${RUST_TARGET}/release/clewdr /app/clewdr
EOF

# 使用 distroless 静态镜像
FROM gcr.io/distroless/static
WORKDIR /app
# 从后端构建阶段复制编译好的二进制文件
COPY --from=backend-builder /app/clewdr .


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
