mod aom;
mod ffmpeg;
mod opus;
#[cfg(target_os = "windows")]
mod zlib;

use crate::utils;
use crate::FFmpegCompiler;
use aom::AomBuilder;
use ffmpeg::FFmpegBuilder;
use opus::OpusBuilder;
use std::io::Result;
use std::process::Command;
#[cfg(target_os = "windows")]
use zlib::ZlibBuilder;

impl FFmpegCompiler {
    /// 统一的构建方法，根据当前平台调用相应的构建函数
    pub fn build_for_platform(&self) -> Result<()> {
        match self.platform.as_str() {
            "macos" => self.build_macos(),
            "windows" => self.build_windows(),
            "linux" => self.build_linux(),
            _ => Err(std::io::Error::other(format!(
                "不支持的平台: {}",
                self.platform
            ))),
        }
    }

    /// Linux 平台构建 FFmpeg
    pub fn build_linux(&self) -> Result<()> {
        let build_dir = self.get_ffmpeg_dir();
        utils::log_info(
            &format!("在 Linux 平台构建 FFmpeg 到 {}", build_dir.display()),
            self.verbose,
        );

        // 检查编译工具
        for tool in &["gcc", "cmake", "make", "pkg-config"] {
            let status = Command::new("which").arg(tool).status()?;

            if !status.success() {
                utils::log_info(&format!("警告: 未找到 {}，可能需要安装", tool), true);
            }
        }

        // 构建 AOM
        self.build_aom_if_needed()?;

        // 构建 Opus
        self.build_opus_if_needed()?;

        // 构建 FFmpeg
        self.build_ffmpeg()
    }

    /// macOS 平台构建 FFmpeg
    pub fn build_macos(&self) -> Result<()> {
        let build_dir = self.get_ffmpeg_dir();
        utils::log_info(
            &format!("在 macOS 平台构建 FFmpeg 到 {}", build_dir.display()),
            self.verbose,
        );

        let mut tools = vec!["cmake", "make", "pkg-config"];
        if cfg!(target_arch = "x86_64") {
            tools.push("nasm");
        }
        for tool in tools {
            if !Command::new("which").arg(tool).status()?.success() {
                return Err(std::io::Error::other(format!(
                    "build-from-source requires {tool} in PATH"
                )));
            }
        }

        // 构建 AOM
        self.build_aom_if_needed()?;

        // 构建 Opus
        self.build_opus_if_needed()?;

        // 构建 FFmpeg
        self.build_ffmpeg()
    }

    /// Windows 平台构建 FFmpeg
    pub fn build_windows(&self) -> Result<()> {
        let build_dir = self.get_ffmpeg_dir();
        utils::log_info(
            &format!("在 Windows 平台构建 FFmpeg 到 {}", build_dir.display()),
            self.verbose,
        );

        utils::log_info("Windows 平台的 FFmpeg 编译需要一些前置条件：", true);
        utils::log_info(
            "1. 安装 LLVM: https://github.com/llvm/llvm-project/releases",
            true,
        );
        utils::log_info("2. 安装 Visual Studio (MSVC)", true);
        utils::log_info("3. 安装 MSYS2: https://www.msys2.org/", true);

        // 构建 AOM
        self.build_aom_if_needed()?;

        // 构建 ZLib
        self.build_zlib_if_needed()?;

        // 构建 Opus
        self.build_opus_if_needed()?;

        // 构建 FFmpeg
        self.build_ffmpeg()
    }

    // 构建单个组件的辅助方法
    fn build_aom_if_needed(&self) -> Result<()> {
        if self.components.contains(&crate::Component::AOM) {
            let source_dir = &self.source_dir;
            let ffmpeg_dir = self.get_ffmpeg_dir();
            let job_count = self.job_count;
            let verbose = self.verbose;

            let aom_builder = AomBuilder::new(source_dir, &ffmpeg_dir, job_count, verbose);
            aom_builder.build()?;
        }
        Ok(())
    }

    fn build_zlib_if_needed(&self) -> Result<()> {
        if self.components.contains(&crate::Component::ZLib) {
            #[cfg(target_os = "windows")]
            {
                let source_dir = &self.source_dir;
                let ffmpeg_dir = self.get_ffmpeg_dir();
                let job_count = self.job_count;
                let verbose = self.verbose;

                let zlib_builder = ZlibBuilder::new(source_dir, &ffmpeg_dir, job_count, verbose);
                zlib_builder.build()?;
            }
        }
        Ok(())
    }

    fn build_opus_if_needed(&self) -> Result<()> {
        if self.components.contains(&crate::Component::Opus) {
            let source_dir = &self.source_dir;
            let ffmpeg_dir = self.get_ffmpeg_dir();
            let job_count = self.job_count;
            let verbose = self.verbose;

            let opus_builder = OpusBuilder::new(source_dir, &ffmpeg_dir, job_count, verbose);
            opus_builder.build()?;
        }
        Ok(())
    }

    fn build_ffmpeg(&self) -> Result<()> {
        let source_dir = &self.source_dir;
        let ffmpeg_dir = self.get_ffmpeg_dir();
        let components = &self.components;
        let audio_codecs = &self.audio_codecs;
        let video_codecs = &self.video_codecs;
        let muxer_formats = &self.muxer_formats;
        let enable_hwaccel = self.enable_hwaccel;
        let job_count = self.job_count;
        let verbose = self.verbose;

        let ffmpeg_builder = FFmpegBuilder::new(
            source_dir,
            &ffmpeg_dir,
            components,
            audio_codecs,
            video_codecs,
            muxer_formats,
            enable_hwaccel,
            job_count,
            verbose,
        );
        ffmpeg_builder.build()?;

        Ok(())
    }
}
