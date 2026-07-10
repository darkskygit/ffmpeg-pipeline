//! FFmpeg 编译模块

use std::collections::HashSet;
#[cfg(target_os = "windows")]
use std::env;
use std::io::Result;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::types::{AudioCodec, Component, MuxerFormat, VideoCodec};
use crate::utils;

pub struct FFmpegBuilder<'a> {
    /// 源码目录
    source_dir: &'a PathBuf,
    /// 输出目录
    output_dir: &'a PathBuf,
    /// 组件列表
    components: &'a HashSet<Component>,
    /// 音频编解码器
    audio_codecs: &'a HashSet<AudioCodec>,
    /// 视频编解码器
    video_codecs: &'a HashSet<VideoCodec>,
    /// 封装格式
    muxer_formats: &'a HashSet<MuxerFormat>,
    /// 是否启用 GPU 硬件加速
    enable_hwaccel: bool,
    /// 编译线程数
    job_count: usize,
    /// 是否输出详细日志
    verbose: bool,
}

impl<'a> FFmpegBuilder<'a> {
    #[expect(
        clippy::too_many_arguments,
        reason = "FFmpeg feature selection is assembled by the outer compiler and passed through explicitly"
    )]
    pub fn new(
        source_dir: &'a PathBuf,
        output_dir: &'a PathBuf,
        components: &'a HashSet<Component>,
        audio_codecs: &'a HashSet<AudioCodec>,
        video_codecs: &'a HashSet<VideoCodec>,
        muxer_formats: &'a HashSet<MuxerFormat>,
        enable_hwaccel: bool,
        job_count: usize,
        verbose: bool,
    ) -> Self {
        Self {
            source_dir,
            output_dir,
            components,
            audio_codecs,
            video_codecs,
            muxer_formats,
            enable_hwaccel,
            job_count,
            verbose,
        }
    }

    /// 获取所有平台通用的 FFmpeg 克隆过程
    fn prepare_ffmpeg_source(&self) -> Result<PathBuf> {
        let branch = "n7.1.5";
        let ffmpeg_dir = self.source_dir.join("ffmpeg");

        if !ffmpeg_dir.exists() {
            utils::clone_repository(
                "https://github.com/ffmpeg/ffmpeg",
                &ffmpeg_dir,
                Some(branch),
                None,
                self.verbose,
            )?;
        }

        Ok(ffmpeg_dir)
    }

    #[cfg(target_os = "windows")]
    fn script_dir(&self) -> PathBuf {
        self.output_dir.join("_build").join("ffmpeg")
    }

    #[cfg(target_os = "windows")]
    fn to_msys_path(path: &Path) -> String {
        let path = path.to_string_lossy().replace('\\', "/");
        if path.len() >= 2 && path.as_bytes()[1] == b':' {
            format!("/{}/{}", &path[..1], &path[2..].trim_start_matches('/'))
        } else {
            path
        }
    }

    /// 获取通用的视频编解码器配置
    fn get_video_codec_config(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut video_decoders = Vec::new();
        let video_encoders = Vec::new();
        let mut extra_args = Vec::new();

        for codec in self.video_codecs {
            match codec {
                VideoCodec::H264 => {
                    video_decoders.push("h264".to_string());
                }
                VideoCodec::HEVC => {
                    video_decoders.push("hevc".to_string());
                }
                VideoCodec::AV1 => {
                    video_decoders.push("libaom_av1".to_string());
                    extra_args.push("--enable-parser=av1".to_string());
                }
                VideoCodec::VP9 => {
                    video_decoders.push("libvpx_vp9".to_string());
                }
            }
        }

        (video_decoders, video_encoders, extra_args)
    }

    /// 获取通用的音频编解码器配置
    fn get_audio_codec_config(&self) -> (Vec<String>, Vec<String>) {
        let mut audio_decoders = Vec::new();
        let mut audio_encoders = Vec::new();

        for codec in self.audio_codecs {
            match codec {
                AudioCodec::Opus => {
                    audio_encoders.push("libopus".to_string());
                }
                AudioCodec::AAC => {
                    audio_decoders.push("aac".to_string());
                }
                AudioCodec::MP3 => {
                    audio_decoders.push("mp3".to_string());
                }
                AudioCodec::FLAC => {
                    audio_decoders.push("flac".to_string());
                }
                AudioCodec::PCM => {
                    audio_decoders.extend(
                        [
                            "pcm_s16le",
                            "pcm_s24le",
                            "pcm_s32le",
                            "pcm_f32le",
                            "pcm_f64le",
                            "pcm_u8",
                        ]
                        .map(str::to_string),
                    );
                }
            }
        }

        (audio_decoders, audio_encoders)
    }

    /// 获取通用的封装格式配置
    fn get_format_config(&self) -> (Vec<String>, Vec<String>) {
        let mut muxers = Vec::new();
        let mut demuxers = Vec::new();
        for format in self.muxer_formats {
            match format {
                MuxerFormat::Matroska => {
                    muxers.push("matroska".to_string());
                    demuxers.push("matroska".to_string());
                }
                MuxerFormat::MP4 => {
                    muxers.push("mp4".to_string());
                    demuxers.push("mov".to_string());
                }
                MuxerFormat::WebM => {
                    muxers.push("webm".to_string());
                    demuxers.push("matroska".to_string());
                }
                MuxerFormat::Ogg => {
                    muxers.push("ogg".to_string());
                    demuxers.push("ogg".to_string());
                }
                MuxerFormat::MOV => {
                    muxers.push("mov".to_string());
                    demuxers.push("mov".to_string());
                }
                MuxerFormat::WAV => {
                    muxers.push("wav".to_string());
                    demuxers.push("wav".to_string());
                }
            }
        }
        if self.audio_codecs.contains(&AudioCodec::AAC) {
            demuxers.push("aac".to_string());
        }
        if self.audio_codecs.contains(&AudioCodec::MP3) {
            demuxers.push("mp3".to_string());
        }
        if self.audio_codecs.contains(&AudioCodec::FLAC) {
            demuxers.push("flac".to_string());
        }
        muxers.sort_unstable();
        muxers.dedup();
        demuxers.sort_unstable();
        demuxers.dedup();
        (muxers, demuxers)
    }

    /// 在 macOS 平台编译 FFmpeg
    #[cfg(target_os = "macos")]
    pub fn build(&self) -> Result<()> {
        utils::log_info("编译 FFmpeg...", self.verbose);
        let ffmpeg_dir = self.prepare_ffmpeg_source()?;

        // 基本配置选项
        let mut configure_args: Vec<String> = vec![
            format!("--prefix={}", self.output_dir.display()),
            "--disable-everything".to_string(),
            "--disable-programs".to_string(),
            "--disable-doc".to_string(),
            "--disable-network".to_string(),
            "--disable-autodetect".to_string(),
            "--disable-shared".to_string(),
            "--enable-static".to_string(),
            "--enable-pic".to_string(),
            "--pkg-config-flags=--static".to_string(),
            format!(
                "--extra-cflags=-I{}",
                self.output_dir.join("include").display()
            ),
            format!(
                "--extra-ldflags=-L{}",
                self.output_dir.join("lib").display()
            ),
            "--enable-gpl".to_string(),
        ];

        // 添加组件配置
        if self.components.contains(&Component::Opus) {
            configure_args.push("--enable-libopus".to_string());
        }
        if self.components.contains(&Component::AOM) {
            configure_args.push("--enable-libaom".to_string());
        }
        if self.components.contains(&Component::ZLib) {
            configure_args.push("--enable-zlib".to_string());
        }

        // 添加封装格式
        let (muxers, demuxers) = self.get_format_config();
        if !muxers.is_empty() {
            configure_args.push(format!("--enable-muxer={}", muxers.join(",")));
        }
        if !demuxers.is_empty() {
            configure_args.push(format!("--enable-demuxer={}", demuxers.join(",")));
        }

        // 添加视频编解码器和音频编解码器
        let (video_decoders, mut video_encoders, extra_args) = self.get_video_codec_config();
        let (audio_decoders, audio_encoders) = self.get_audio_codec_config();

        // 合并额外参数
        configure_args.extend(extra_args);

        // 如果启用硬件加速, 添加特定编码器
        if self.enable_hwaccel {
            if self.video_codecs.contains(&VideoCodec::H264) {
                video_encoders.push("h264_videotoolbox".to_string());
            }
            if self.video_codecs.contains(&VideoCodec::HEVC) {
                video_encoders.push("hevc_videotoolbox".to_string());
            }
        }

        // 合并音视频编解码器
        let mut all_decoders = video_decoders.clone();
        all_decoders.extend(audio_decoders);

        let mut all_encoders = video_encoders;
        all_encoders.extend(audio_encoders);

        if !all_decoders.is_empty() {
            configure_args.push(format!("--enable-decoder={}", all_decoders.join(",")));
        }

        if !all_encoders.is_empty() {
            configure_args.push(format!("--enable-encoder={}", all_encoders.join(",")));
        }

        // 添加协议
        configure_args.push("--enable-protocol=file,data,pipe".to_string());

        // 添加硬件加速
        if self.enable_hwaccel {
            configure_args.push("--enable-hwaccel=h264_videotoolbox,hevc_videotoolbox".to_string());
        }

        // 添加基础滤镜
        configure_args.push("--enable-filter=anull,aresample".to_string());

        // 添加其他选项
        configure_args.push("--enable-small".to_string());
        configure_args.push("--extra-cflags=\"-stdlib=libc++\"".to_string());
        configure_args.push("--extra-cxxflags=\"-stdlib=libc++\"".to_string());

        // 输出配置命令
        utils::log_info(
            &format!("FFmpeg 配置命令: ./configure {}", configure_args.join(" ")),
            self.verbose,
        );

        // 执行配置命令
        let configure_output = Command::new("./configure")
            .current_dir(&ffmpeg_dir)
            .env(
                "PKG_CONFIG_PATH",
                self.output_dir.join("lib").join("pkgconfig"),
            )
            .args(&configure_args)
            .output()?;
        utils::handle_command_output(Ok(configure_output), "配置 FFmpeg")?;

        // 编译 FFmpeg
        let make_output = Command::new("make")
            .current_dir(&ffmpeg_dir)
            .args(["-j", &self.job_count.to_string()])
            .output()?;
        utils::handle_command_output(Ok(make_output), "编译 FFmpeg")?;

        // 安装 FFmpeg
        let install_output = Command::new("make")
            .current_dir(&ffmpeg_dir)
            .arg("install")
            .output()?;
        utils::handle_command_output(Ok(install_output), "安装 FFmpeg")?;

        utils::log_success("FFmpeg 编译完成", self.verbose);
        Ok(())
    }

    /// 在 Linux 平台编译 FFmpeg
    #[cfg(target_os = "linux")]
    pub fn build(&self) -> Result<()> {
        utils::log_info("编译 FFmpeg...", self.verbose);
        let ffmpeg_dir = self.prepare_ffmpeg_source()?;

        // 基本配置选项
        let mut configure_args: Vec<String> = vec![
            format!("--prefix={}", self.output_dir.display()),
            "--disable-everything".to_string(),
            "--disable-programs".to_string(),
            "--disable-doc".to_string(),
            "--disable-network".to_string(),
            "--disable-autodetect".to_string(),
            "--disable-shared".to_string(),
            "--enable-static".to_string(),
            "--enable-pic".to_string(),
            "--pkg-config-flags=--static".to_string(),
            format!(
                "--extra-cflags=-I{}",
                self.output_dir.join("include").display()
            ),
            format!(
                "--extra-ldflags=-L{}",
                self.output_dir.join("lib").display()
            ),
            "--enable-gpl".to_string(),
        ];

        // 添加组件配置
        if self.components.contains(&Component::Opus) {
            configure_args.push("--enable-libopus".to_string());
        }
        if self.components.contains(&Component::AOM) {
            configure_args.push("--enable-libaom".to_string());
        }
        if self.components.contains(&Component::ZLib) {
            configure_args.push("--enable-zlib".to_string());
        }

        // 添加封装格式
        let (muxers, demuxers) = self.get_format_config();
        if !muxers.is_empty() {
            configure_args.push(format!("--enable-muxer={}", muxers.join(",")));
        }
        if !demuxers.is_empty() {
            configure_args.push(format!("--enable-demuxer={}", demuxers.join(",")));
        }

        // 添加视频编解码器和音频编解码器
        let (video_decoders, video_encoders, extra_args) = self.get_video_codec_config();
        let (audio_decoders, audio_encoders) = self.get_audio_codec_config();

        // 合并额外参数
        configure_args.extend(extra_args);

        // 合并音视频编解码器
        let mut all_decoders = video_decoders;
        all_decoders.extend(audio_decoders);

        let mut all_encoders = video_encoders;
        all_encoders.extend(audio_encoders);

        if !all_decoders.is_empty() {
            configure_args.push(format!("--enable-decoder={}", all_decoders.join(",")));
        }

        if !all_encoders.is_empty() {
            configure_args.push(format!("--enable-encoder={}", all_encoders.join(",")));
        }

        // 添加协议
        configure_args.push("--enable-protocol=file,data,pipe".to_string());

        // 添加硬件加速
        if self.enable_hwaccel {
            configure_args.push("--enable-hwaccel=h264_vaapi,hevc_vaapi".to_string());
        }

        // 添加基础滤镜
        configure_args.push("--enable-filter=anull,aresample".to_string());

        // 添加其他选项
        configure_args.push("--enable-small".to_string());

        // 输出配置命令
        utils::log_info(
            &format!("FFmpeg 配置命令: ./configure {}", configure_args.join(" ")),
            self.verbose,
        );

        // 执行配置命令
        let configure_output = Command::new("./configure")
            .current_dir(&ffmpeg_dir)
            .env(
                "PKG_CONFIG_PATH",
                self.output_dir.join("lib").join("pkgconfig"),
            )
            .args(&configure_args)
            .output()?;
        utils::handle_command_output(Ok(configure_output), "配置 FFmpeg")?;

        // 编译 FFmpeg
        let make_output = Command::new("make")
            .current_dir(&ffmpeg_dir)
            .args(&["-j", &self.job_count.to_string()])
            .output()?;
        utils::handle_command_output(Ok(make_output), "编译 FFmpeg")?;

        // 安装 FFmpeg
        let install_output = Command::new("make")
            .current_dir(&ffmpeg_dir)
            .arg("install")
            .output()?;
        utils::handle_command_output(Ok(install_output), "安装 FFmpeg")?;

        utils::log_success("FFmpeg 编译完成", self.verbose);
        Ok(())
    }

    /// 在 Windows 平台编译 FFmpeg
    #[cfg(target_os = "windows")]
    pub fn build(&self) -> Result<()> {
        use std::fs;

        utils::log_info("编译 FFmpeg...", self.verbose);
        utils::log_info("注意：编译 FFmpeg 在 Windows 上需要 MSYS2 环境", true);

        let ffmpeg_dir = self.prepare_ffmpeg_source()?;
        utils::mkdir(self.output_dir)?;
        let script_dir = self.script_dir();
        utils::mkdir(&script_dir)?;
        let output_dir_msys = Self::to_msys_path(self.output_dir);
        let ffmpeg_dir_msys = Self::to_msys_path(&ffmpeg_dir);
        let pkg_config_path_msys =
            Self::to_msys_path(&self.output_dir.join("lib").join("pkgconfig"));

        // 构建 FFmpeg 配置选项
        let mut configure_options = String::new();

        // 基本配置
        configure_options.push_str(&format!("--prefix={} ", output_dir_msys));
        configure_options.push_str("--disable-everything ");
        configure_options.push_str("--disable-programs ");
        configure_options.push_str("--disable-doc ");
        configure_options.push_str("--disable-network ");
        configure_options.push_str("--disable-autodetect ");
        configure_options.push_str("--disable-shared ");
        configure_options.push_str("--enable-static ");
        configure_options.push_str("--enable-pic ");
        configure_options.push_str("--pkg-config-flags=--static ");
        configure_options.push_str("--enable-gpl ");

        // 添加组件配置
        if self.components.contains(&Component::Opus) {
            configure_options.push_str("--enable-libopus ");
        }
        if self.components.contains(&Component::AOM) {
            configure_options.push_str("--enable-libaom ");
        }
        if self.components.contains(&Component::ZLib) {
            configure_options.push_str("--enable-zlib ");
        }

        // 添加封装格式
        let (muxers, demuxers) = self.get_format_config();
        if !muxers.is_empty() {
            configure_options.push_str(&format!("--enable-muxer={} ", muxers.join(",")));
        }
        if !demuxers.is_empty() {
            configure_options.push_str(&format!("--enable-demuxer={} ", demuxers.join(",")));
        }

        // 添加视频编解码器和音频编解码器
        let (mut decoders, mut encoders, _) = self.get_video_codec_config();
        let (audio_decoders, audio_encoders) = self.get_audio_codec_config();

        // 合并音视频编解码器
        decoders.extend(audio_decoders);
        encoders.extend(audio_encoders);

        // 添加AV1解析器
        if self.video_codecs.contains(&VideoCodec::AV1) {
            configure_options.push_str("--enable-parser=av1 ");
        }

        if !decoders.is_empty() {
            configure_options.push_str(&format!("--enable-decoder={} ", decoders.join(",")));
        }

        if !encoders.is_empty() {
            configure_options.push_str(&format!("--enable-encoder={} ", encoders.join(",")));
        }

        // 添加协议
        configure_options.push_str("--enable-protocol=file,data,pipe ");

        // 添加硬件加速
        if self.enable_hwaccel {
            configure_options.push_str("--enable-hwaccel=h264_d3d11va,h264_d3d11va2,h264_dxva2,hevc_d3d11va,hevc_d3d11va2,hevc_dxva2,av1_d3d11va,av1_d3d11va2,av1_dxva2 ");
        }

        // 添加基础滤镜
        configure_options.push_str("--enable-filter=anull,aresample ");

        // 添加 Windows 特定选项
        configure_options.push_str("--arch=x86_64 ");
        configure_options.push_str("--target-os=win64 ");
        configure_options.push_str("--toolchain=msvc ");
        configure_options.push_str("--pkg-config=pkg-config ");

        // 添加其他选项
        configure_options.push_str("--enable-small ");

        // 创建 MSYS2 构建脚本
        let msys_script = format!(
            r#"
#!/bin/bash
set -e
export PKG_CONFIG_PATH="{}:$PKG_CONFIG_PATH"
mkdir -p "{}"
cd "{}"
if ! ./configure {}; then
  cat ffbuild/config.log
  exit 1
fi
make -B -j{}
make install
"#,
            pkg_config_path_msys,
            output_dir_msys,
            ffmpeg_dir_msys,
            configure_options,
            self.job_count
        );

        let build_script_path = script_dir.join("build_ffmpeg.sh");
        fs::write(&build_script_path, &msys_script)?;

        // 创建执行 MSYS2 的批处理文件
        let msys2_root = env::var("MSYS2_LOCATION").unwrap_or_else(|_| r"C:\msys64".into());
        let msys_launcher = PathBuf::from(msys2_root).join("msys2_shell.cmd");
        let include_dir = self.output_dir.join("include");
        let lib_dir = self.output_dir.join("lib");
        let msys_cmd = format!(
            r#"
@echo off
set MSYS2_PATH_TYPE=inherit
set "INCLUDE={};%INCLUDE%"
set "LIB={};%LIB%"
call "{}" -defterm -here -no-start -mingw64 -c "chmod +x build_ffmpeg.sh && ./build_ffmpeg.sh"
"#,
            include_dir.display(),
            lib_dir.display(),
            msys_launcher.display()
        );

        let run_script_path = script_dir.join("run_msys.bat");
        fs::write(&run_script_path, msys_cmd)?;

        // 运行批处理文件
        let msys_output = Command::new("cmd")
            .current_dir(&script_dir)
            .args(["/C", "run_msys.bat"])
            .output()?;
        utils::handle_command_output(Ok(msys_output), "编译 FFmpeg")?;

        fs::remove_file(build_script_path)?;
        fs::remove_file(run_script_path)?;

        utils::log_success("FFmpeg 编译完成", self.verbose);
        Ok(())
    }
}
