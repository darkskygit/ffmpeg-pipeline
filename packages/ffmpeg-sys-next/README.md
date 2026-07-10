# ffmpeg-sys-next workspace fork

This directory vendors [`ffmpeg-sys-next`](https://crates.io/crates/ffmpeg-sys-next) and carries repository-specific changes for static SDK selection. It is not intended to replace or be published over the upstream crate.

## Repository-specific behavior

When this fork is injected through `[patch.crates-io]`, the SDK provider selects FFmpeg in this order:

1. Use the SDK at `FFMPEG_DIR` when the variable is set.
2. Build FFmpeg and its configured dependencies when the `build` feature is enabled.
3. Otherwise, download the pinned SDK release for the current target and verify its SHA-256 checksum.

Published SDKs are available for:

- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Windows MSVC builds require a static C runtime. Set `RUSTFLAGS="-C target-feature=+crt-static"`; the build script rejects mixed static-FFmpeg/dynamic-CRT builds.

The provider recognizes these variables:

- `FFMPEG_DIR`: path to a compatible SDK containing `include/` and `lib/`.
- `FFMPEG_SDK_TAG`: override the pinned SDK release tag.
- `FFMPEG_SDK_BASE_URL`: override the release download base URL.
- `CMAKE_GENERATOR`: override the CMake generator used for source builds.

Use the fork from a workspace root because Cargo does not propagate patches declared by dependencies:

```toml
[patch.crates-io]
ffmpeg-sys-next = { git = "https://github.com/darkskygit/ffmpeg-pipeline", tag = "<release-tag>" }
```

## Upstream crate

The vendored crate contains low-level FFmpeg bindings and follows the upstream `ffmpeg-sys-next` versioning and feature model. Most applications should use a higher-level wrapper such as [`ffmpeg-next`](https://github.com/zmwangx/rust-ffmpeg).

Besides the features declared in `Cargo.toml`, the build script exposes compile-time FFmpeg version and API detection flags, including:

- `ffmpeg_<major>_<minor>` for detected FFmpeg versions.
- `avcodec_version_greater_than_<major>_<minor>` for libavcodec version checks.
- `ff_api_<feature>` and `ff_api_<feature>_is_defined` for FFmpeg compatibility guards.

Run `cargo build -vv` to inspect the complete set emitted for a particular SDK.
