#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build_root="${FFMPEG_BUILD_ROOT:-$root/.build/ffmpeg-sdk}"
prefix="${FFMPEG_SDK_DIR:-$root/ffmpeg-sdk}"
ffmpeg_version="${FFMPEG_VERSION:-7.1.5}"
opus_version="${OPUS_VERSION:-1.5.2}"
aom_version="${AOM_VERSION:-3.12.1}"

rm -rf "$build_root" "$prefix"
mkdir -p "$build_root" "$prefix"

if [[ "${RUNNER_OS:-}" != "Windows" && "$(uname -s)" != MINGW* ]]; then
  export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
fi

git clone --depth 1 --branch "v$opus_version" https://github.com/xiph/opus.git "$build_root/opus"
git clone --depth 1 --branch "v$aom_version" https://aomedia.googlesource.com/aom "$build_root/aom"

if [[ "${RUNNER_OS:-}" == "Windows" || "$(uname -s)" == MINGW* ]]; then
  cmake -S "$build_root/opus" -B "$build_root/opus-build" -G "Visual Studio 17 2022" -A x64 \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded \
    -DOPUS_BUILD_SHARED_LIBRARY=OFF \
    -DOPUS_BUILD_TESTING=OFF \
    -DOPUS_BUILD_PROGRAMS=OFF
  cmake --build "$build_root/opus-build" --config Release --target install

  cmake -S "$build_root/aom" -B "$build_root/aom-build" -G "Visual Studio 17 2022" -A x64 \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded \
    -DBUILD_SHARED_LIBS=OFF \
    -DENABLE_DOCS=OFF \
    -DENABLE_EXAMPLES=OFF \
    -DENABLE_TESTDATA=OFF \
    -DENABLE_TESTS=OFF \
    -DENABLE_TOOLS=OFF \
    -DCONFIG_AV1_ENCODER=0
  cmake --build "$build_root/aom-build" --config Release --target install

  git clone --depth 1 --branch v1.3.1 https://github.com/madler/zlib.git "$build_root/zlib"
  cmake -S "$build_root/zlib" -B "$build_root/zlib-build" -G "Visual Studio 17 2022" -A x64 \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded \
    -DBUILD_SHARED_LIBS=OFF
  cmake --build "$build_root/zlib-build" --config Release --target install
  if [[ -f "$prefix/lib/zlibstatic.lib" && ! -f "$prefix/lib/z.lib" ]]; then
    cp "$prefix/lib/zlibstatic.lib" "$prefix/lib/z.lib"
  fi
else
  cmake -S "$build_root/opus" -B "$build_root/opus-build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DOPUS_BUILD_SHARED_LIBRARY=OFF \
    -DOPUS_BUILD_TESTING=OFF \
    -DOPUS_BUILD_PROGRAMS=OFF
  cmake --build "$build_root/opus-build" --parallel
  cmake --install "$build_root/opus-build"

  cmake -S "$build_root/aom" -B "$build_root/aom-build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$prefix" \
    -DBUILD_SHARED_LIBS=OFF \
    -DENABLE_DOCS=OFF \
    -DENABLE_EXAMPLES=OFF \
    -DENABLE_TESTDATA=OFF \
    -DENABLE_TESTS=OFF \
    -DENABLE_TOOLS=OFF \
    -DCONFIG_AV1_ENCODER=0
  cmake --build "$build_root/aom-build" --parallel
  cmake --install "$build_root/aom-build"
fi

git clone --depth 1 --branch "n$ffmpeg_version" https://github.com/FFmpeg/FFmpeg.git "$build_root/ffmpeg"

export PKG_CONFIG_PATH="$prefix/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
common_config=(
  "--prefix=$prefix"
  --disable-programs
  --disable-doc
  --disable-debug
  --disable-network
  --disable-autodetect
  --disable-shared
  --enable-static
  --enable-pic
  --enable-libopus
  --enable-libaom
  --disable-decoder=av1
  --disable-encoder=libaom_av1
  --enable-zlib
  --pkg-config-flags=--static
  "--extra-cflags=-I$prefix/include"
)

cd "$build_root/ffmpeg"
if [[ "${RUNNER_OS:-}" == "Windows" || "$(uname -s)" == MINGW* ]]; then
  export PATH="$(cygpath "$VCToolsInstallDir")/bin/Hostx64/x64:$PATH"
  ./configure "${common_config[@]}" \
    --arch=x86_64 \
    --target-os=win64 \
    --toolchain=msvc \
    "--extra-ldflags=-LIBPATH:$(cygpath -w "$prefix/lib")"
else
  ./configure "${common_config[@]}" "--extra-ldflags=-L$prefix/lib"
fi

make -j"${NUMBER_OF_PROCESSORS:-$(sysctl -n hw.logicalcpu 2>/dev/null || getconf _NPROCESSORS_ONLN)}"
make install

for pc in "$prefix"/lib/pkgconfig/*.pc; do
  sed -i.bak 's|^prefix=.*|prefix=${pcfiledir}/../..|' "$pc"
  rm -f "$pc.bak"
done
