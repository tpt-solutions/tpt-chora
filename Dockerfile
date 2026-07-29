FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libvulkan-dev \
    mesa-vulkan-drivers \
    vulkan-tools \
    libgl1-mesa-dri \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && rustup default stable \
    && rustup component add rustfmt clippy

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --workspace --all-targets

RUN cargo test --workspace --all-targets

CMD ["cargo", "test", "--workspace", "--all-targets"]