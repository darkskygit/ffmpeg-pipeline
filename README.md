# ffmpeg-pipeline

[![Crates.io](https://img.shields.io/crates/v/ffmpeg-pipeline.svg)](https://crates.io/crates/ffmpeg-pipeline)
[![Documentation](https://docs.rs/ffmpeg-pipeline/badge.svg)](https://docs.rs/ffmpeg-pipeline)
[![CI](https://github.com/darkskygit/ffmpeg-pipeline/actions/workflows/ffmpeg-sdk.yml/badge.svg)](https://github.com/darkskygit/ffmpeg-pipeline/actions/workflows/ffmpeg-sdk.yml)
[![License](https://img.shields.io/badge/license-AGPL--3.0--only-blue.svg)](LICENSE)

`ffmpeg-pipeline` provides composable Rust APIs for common media-processing workflows on top of [`ffmpeg-next`](https://crates.io/crates/ffmpeg-next). It operates directly through FFmpeg libraries and never invokes the `ffmpeg` command-line program.

## Features

- Decode audio and video streams as packets or frames.
- Encode audio and video with configurable codec parameters and metadata.
- Resample, reformat, and buffer audio for encoder frame requirements.
- Scale decoded video frames.
- Remux selected streams while preserving or replacing stream metadata.
- Read from files, byte buffers, or custom `Read + Seek` sources.
- Write to files, byte buffers, or custom `Write + Seek` destinations.
- Inspect stream metadata and calculate video frame counts.

## Installation

```toml
[dependencies]
ffmpeg-pipeline = "0.1"
log = "0.4"
```

The crate links FFmpeg statically. Provide a compatible FFmpeg 7.1 installation through `FFMPEG_DIR`, or enable source compilation:

```toml
[dependencies]
ffmpeg-pipeline = { version = "0.1", features = ["build-from-source"] }
```

The available codecs and formats are determined by the linked FFmpeg build. Platform system libraries and frameworks may still be required; third-party FFmpeg dependencies must be linked statically.

## Example

```rust,no_run
use ffmpeg_pipeline::{initialize, input_file, parse_stream_info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize(log::Level::Error)?;

    let input = input_file("sample.mkv")?;
    for stream in input.streams() {
        let info = parse_stream_info(&stream)?;
        println!("{}: {} ({})", stream.index(), info.get_title(), info.format);
    }

    Ok(())
}
```

Call `initialize` once before creating FFmpeg contexts. See the [API documentation](https://docs.rs/ffmpeg-pipeline) for decoding, encoding, remuxing, scaling, and custom I/O types.

## Status

The crate is under active development. The public API may change before version 1.0.

## License

Copyright © 2023–2026 DarkSky.

Licensed under the [GNU Affero General Public License, version 3 only](LICENSE).
