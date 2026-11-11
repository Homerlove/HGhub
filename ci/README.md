# CI runner image

`ci/Dockerfile` 定义 HghubX 的 GitHub Actions 运行镜像：

- 基础镜像：`mcr.microsoft.com/playwright:v1.63.0-noble`
- 预装：Chromium、ffmpeg、Rust stable、pnpm 10
- Tauri/Linux 依赖：webkit2gtk 4.1、GTK3、ayatana-appindicator、librsvg、patchelf、libxdo

`.github/workflows/ci.yml` 的 `image` job 会用 Dockerfile 内容哈希作为 tag
（例如 `ghcr.io/zxsos/hghub-ci:<hash>`）；首次构建后推送到 GHCR，后续 CI
直接 `docker manifest inspect` 命中并跳过构建，frontend / rust job 从该镜像启动。
