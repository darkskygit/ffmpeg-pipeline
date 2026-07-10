download zlib & libopus & ffmpeg source code

install llvm: https://github.com/llvm/llvm-project/releases

call x64 visual studio command prompt

```bat
git clone https://aomedia.googlesource.com/aom
md aom_build
cd aom_build
cmake -G "Visual Studio 17 2022" -DCMAKE_INSTALL_PREFIX=../ffmpeg_build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DENABLE_DOCS=OFF -DENABLE_EXAMPLES=OFF -DENABLE_TESTS=OFF -DENABLE_TESTDATA=OFF -DENABLE_TOOLS=OFF -DENABLE_NASM=on -DCMAKE_C_FLAGS_RELEASE="/MT /GL" -DCMAKE_CXX_FLAGS_RELEASE="/MT /GL" -DCMAKE_MSVC_RUNTIME_LIBRARY="MultiThreaded$<$<CONFIG:Debug>:Debug>" ../aom
cmake --build . --config Release --target install
```

patch zlib:
https://github.com/microsoft/vcpkg/blob/master/ports/zlib/0001-Prevent-invalid-inclusions-when-HAVE_-is-set-to-0.patch

```bat
cd zlib-1.3
nmake -f win32/Makefile.msc
```

download libopus: https://opus-codec.org/downloads/

```bat
md opus_build
cd opus_build
cmake -G "Visual Studio 17 2022" -DCMAKE_INSTALL_PREFIX=../ffmpeg_build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DOPUS_OSCE=ON -DOPUS_STATIC_RUNTIME=ON -DCMAKE_C_FLAGS_RELEASE="/MT /GL" -DCMAKE_CXX_FLAGS_RELEASE="/MT /GL" ../opus-1.5.2
cmake --build . --config Release --target install
```

run windows_msys.bat to start msys2
cd to ffmpeg source code directory

```sh
export PATH="/C/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.41.34120/bin/Hostx64/x64":$PATH
export PKG_CONFIG_PATH=$PKG_CONFIG_PATH:$PWD/../ffmpeg_build/lib/pkgconfig
pacman -S pkg-config yasm diffutils
mkdir ../ffmpeg_build
./configure --prefix=../ffmpeg_build --disable-everything --disable-programs --disable-doc --enable-gpl --enable-libaom --enable-libopus --enable-muxer=matroska,mp4,ogg,opus,mov --enable-demuxer=matroska,mp4,ogg,opus,mov --enable-encoder=libopus --enable-decoder=h264,hevc,libaom_av1,png,aac --enable-parser=av1 --enable-zlib --enable-protocol=file,data,pipe --enable-hwaccel=h264_d3d11va,h264_d3d11va2,h264_dxva2,hevc_d3d11va,hevc_d3d11va2,hevc_dxva2,av1_d3d11va,av1_d3d11va2,av1_dxva2 --enable-filter=anull,aresample --enable-small --arch=x86_64 --target-os=win64 --toolchain=msvc --pkg-config=pkg-config --extra-cflags="-I$PWD/../zlib-1.3" --extra-ldflags="-LIBPATH:$PWD/../zlib-1.3"
make -B -j16 && make install
```

```bat
set FFMPEG_DIR=%cd%\..\ffmpeg_build
```

```powershell
$env:FFMPEG_DIR = "$pwd\..\ffmpeg_build"
```
