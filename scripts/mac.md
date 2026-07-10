```bash
brew install llvm ffmpeg zlib
cd packages/encoder
./install/mac.rs
export FFMPEG_DIR=${PWD}/tmp/ffmpeg_build
cd ../..
cargo build
```
