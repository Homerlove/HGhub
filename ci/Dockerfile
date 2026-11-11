# HghubX CI runner image
# 预装 Playwright/Chromium、ffmpeg、Rust 工具链和 Tauri/Linux 系统依赖，
# 供 .github/workflows/ci.yml 的 frontend / rust job 直接拉取运行。
FROM mcr.microsoft.com/playwright:v1.63.0-noble

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_TERM_COLOR=always \
    PATH=/root/.cargo/bin:$PATH

RUN set -eux; \
    echo '--- apt sources before ---'; \
    cat /etc/apt/sources.list.d/ubuntu.sources; \
    sed -i \
        -e 's|http://azure.archive.ubuntu.com/ubuntu|https://mirrors.tuna.tsinghua.edu.cn/ubuntu|g' \
        -e 's|http://archive.ubuntu.com/ubuntu|https://mirrors.tuna.tsinghua.edu.cn/ubuntu|g' \
        -e 's|http://security.ubuntu.com/ubuntu|https://mirrors.tuna.tsinghua.edu.cn/ubuntu|g' \
        /etc/apt/sources.list.d/ubuntu.sources; \
    echo '--- apt sources after ---'; \
    cat /etc/apt/sources.list.d/ubuntu.sources; \
    apt-get -o Acquire::Retries=3 update; \
    apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        curl \
        wget \
        file \
        git \
        ffmpeg \
        libssl-dev \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        patchelf \
        libxdo-dev; \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustc --version \
    && cargo --version

RUN npm install -g pnpm@10 \
    && pnpm --version
