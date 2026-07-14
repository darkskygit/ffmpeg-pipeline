#[cfg(feature = "build-from-source")]
use std::{collections::HashSet, env, fs, io::Result, path::PathBuf, thread};

#[cfg(feature = "build-from-source")]
mod builder;
#[cfg(feature = "build-from-source")]
mod deps;
#[cfg(feature = "build-from-source")]
mod error;
#[cfg(feature = "build-from-source")]
mod platform; // 提供平台检测
mod prebuilt;
#[cfg(feature = "build-from-source")]
mod preset;
#[cfg(feature = "build-from-source")]
mod types;
#[cfg_attr(not(feature = "build-from-source"), allow(dead_code))]
mod utils;

#[cfg(feature = "build-from-source")]
pub use builder::FFmpegBuilder;
#[cfg(feature = "build-from-source")]
pub use error::FFmpegError;
pub use prebuilt::{ensure_prebuilt_sdk, is_valid_sdk};
#[cfg(feature = "build-from-source")]
pub use preset::pipeline_sdk_builder;
#[cfg(feature = "build-from-source")]
pub use types::{AudioCodec, Component, MuxerFormat, VideoCodec};

/// FFmpeg 编译器，用于管理 FFmpeg 及其依赖的编译过程
#[cfg(feature = "build-from-source")]
pub struct FFmpegCompiler {
    /// 编译输出目录
    build_dir: PathBuf,
    /// 源码目录
    source_dir: PathBuf,
    /// 平台类型
    platform: String,
    /// 组件列表
    components: HashSet<Component>,
    /// 音频编解码器
    audio_codecs: HashSet<AudioCodec>,
    /// 视频编解码器
    video_codecs: HashSet<VideoCodec>,
    /// 封装格式
    muxer_formats: HashSet<MuxerFormat>,
    /// 是否启用 GPU 硬件加速
    enable_hwaccel: bool,
    /// 编译线程数
    job_count: usize,
    /// 是否启用缓存
    use_cache: bool,
    /// 是否启用详细日志
    verbose: bool,
}

/// 编译结果输出迭代器
#[cfg(feature = "build-from-source")]
pub struct FFmpegCompileOutput {
    link_search_paths: Vec<PathBuf>,
    dynamic_link_libs: Vec<String>,
    link_libs: Vec<String>,
    rustc_flags: Vec<String>,
}

#[cfg(feature = "build-from-source")]
impl Iterator for FFmpegCompileOutput {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.rustc_flags.is_empty() {
            return Some(self.rustc_flags.remove(0));
        }

        if !self.link_search_paths.is_empty() {
            let path = self.link_search_paths.remove(0);
            return Some(format!("cargo:rustc-link-search=native={}", path.display()));
        }

        if !self.link_libs.is_empty() {
            let lib = self.link_libs.remove(0);
            #[cfg(target_os = "windows")]
            return Some(format!("cargo:rustc-link-lib=static={}", lib));

            #[cfg(target_os = "macos")]
            return Some(format!("cargo:rustc-link-lib={}", lib));
        }

        if !self.dynamic_link_libs.is_empty() {
            let lib = self.dynamic_link_libs.remove(0);
            return Some(format!("cargo:rustc-link-lib={}", lib));
        }

        None
    }
}

#[cfg(feature = "build-from-source")]
impl FFmpegCompiler {
    /// 创建一个新的 FFmpeg 编译器
    #[expect(
        clippy::too_many_arguments,
        reason = "The public compiler constructor mirrors the top-level builder state one-to-one"
    )]
    pub fn new(
        build_dir: PathBuf,
        source_dir: PathBuf,
        components: HashSet<Component>,
        audio_codecs: HashSet<AudioCodec>,
        video_codecs: HashSet<VideoCodec>,
        muxer_formats: HashSet<MuxerFormat>,
        enable_hwaccel: bool,
        job_count: Option<usize>,
        use_cache: bool,
        verbose: bool,
    ) -> Self {
        let job_count =
            job_count.unwrap_or_else(|| thread::available_parallelism().map_or(2, |p| p.get()));

        Self {
            build_dir,
            source_dir,
            platform: platform::detect_platform(),
            components,
            audio_codecs,
            video_codecs,
            muxer_formats,
            enable_hwaccel,
            job_count,
            use_cache,
            verbose,
        }
    }

    /// 编译 FFmpeg 及其依赖
    pub fn compile(&self) -> Result<FFmpegCompileOutput> {
        utils::log_info("开始编译 FFmpeg 及其依赖", self.verbose);

        // 创建目录
        utils::mkdir(&self.build_dir)?;
        utils::mkdir(&self.source_dir)?;

        // 如果设置了环境变量 FFMPEG_DIR，则跳过编译
        if let Some(ffmpeg_dir) = self.check_ffmpeg_dir() {
            return self.generate_output(ffmpeg_dir);
        }

        // 检查缓存
        if self.use_cache && self.check_cache() {
            utils::log_info("缓存有效，跳过编译步骤", self.verbose);
            return self.generate_output(self.get_ffmpeg_dir());
        }

        // 根据平台执行编译
        self.build_for_platform()?;

        // 更新缓存
        if self.use_cache {
            self.update_cache()?;
        }

        self.generate_output(self.get_ffmpeg_dir())
    }

    // 其他方法将在下面的文件中实现

    /// 检查 FFMPEG_DIR 环境变量
    fn check_ffmpeg_dir(&self) -> Option<PathBuf> {
        env::var("FFMPEG_DIR").ok().map(|path| {
            utils::log_info(
                &format!("检测到 FFMPEG_DIR 环境变量: {}", path),
                self.verbose,
            );
            PathBuf::from(path)
        })
    }

    /// 获取 FFmpeg 输出目录
    fn get_ffmpeg_dir(&self) -> PathBuf {
        self.build_dir.join("ffmpeg_build")
    }

    /// 生成编译输出
    fn generate_output(&self, ffmpeg_dir: PathBuf) -> Result<FFmpegCompileOutput> {
        let link_search_paths = vec![ffmpeg_dir.join("lib")];

        let mut link_libs = vec![
            "avcodec".to_string(),
            "avformat".to_string(),
            "avfilter".to_string(),
            "avdevice".to_string(),
            "avutil".to_string(),
            "swscale".to_string(),
            "swresample".to_string(),
        ];

        // 添加组件对应的库
        for component in &self.components {
            match component {
                Component::Opus => link_libs.push("opus".to_string()),
                Component::AOM => link_libs.push("aom".to_string()),
                Component::ZLib => {
                    if cfg!(target_os = "windows") {
                        link_libs.push("zlib".to_string())
                    }
                }
            }
        }

        let mut dynamic_link_libs = vec![];

        #[cfg(target_os = "macos")]
        {
            dynamic_link_libs.push("c++".to_string());
            dynamic_link_libs.push("bz2".to_string());
            dynamic_link_libs.push("z".to_string());
            if self.enable_hwaccel {
                dynamic_link_libs.extend(
                    [
                        "framework=CoreFoundation",
                        "framework=AudioToolbox",
                        "framework=CoreMedia",
                        "framework=CoreVideo",
                        "framework=Security",
                        "framework=VideoToolbox",
                    ]
                    .map(str::to_string),
                );
            }
        }

        #[cfg(target_os = "windows")]
        {
            dynamic_link_libs.push("user32".to_string());
            dynamic_link_libs.push("ole32".to_string());
        }

        let rustc_flags = vec![
            format!("cargo:rustc-env=FFMPEG_DIR={}", ffmpeg_dir.display()),
            format!("cargo:library={}", ffmpeg_dir.display()),
        ];

        Ok(FFmpegCompileOutput {
            link_search_paths,
            dynamic_link_libs,
            link_libs,
            rustc_flags,
        })
    }

    /// 检查缓存是否有效
    fn check_cache(&self) -> bool {
        utils::check_cache(&self.build_dir, &self.source_dir)
            && fs::read_to_string(self.build_dir.join(".ffmpeg_build_config"))
                .is_ok_and(|cached| cached == self.cache_signature())
    }

    /// 更新缓存
    fn update_cache(&self) -> Result<()> {
        utils::update_cache(&self.build_dir)?;
        fs::write(
            self.build_dir.join(".ffmpeg_build_config"),
            self.cache_signature(),
        )
    }

    fn cache_signature(&self) -> String {
        fn sorted_debug<T: std::fmt::Debug>(values: &HashSet<T>) -> String {
            let mut values = values
                .iter()
                .map(|value| format!("{value:?}"))
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.join(",")
        }

        format!(
            "builder_revision=3\nplatform={}\ncomponents={}\naudio_codecs={}\nvideo_codecs={}\nmuxer_formats={}\nhwaccel={}\n",
            self.platform,
            sorted_debug(&self.components),
            sorted_debug(&self.audio_codecs),
            sorted_debug(&self.video_codecs),
            sorted_debug(&self.muxer_formats),
            self.enable_hwaccel,
        )
    }

    // 平台特定的构建方法在各自的模块文件中实现
}
