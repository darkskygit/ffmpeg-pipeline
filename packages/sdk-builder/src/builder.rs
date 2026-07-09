use crate::types::{AudioCodec, Component, MuxerFormat, VideoCodec};
use crate::FFmpegCompiler;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// FFmpeg 编译器构建器
pub struct FFmpegBuilder {
    build_dir: Option<PathBuf>,
    source_dir: Option<PathBuf>,
    components: HashSet<Component>,
    audio_codecs: HashSet<AudioCodec>,
    video_codecs: HashSet<VideoCodec>,
    muxer_formats: HashSet<MuxerFormat>,
    enable_hwaccel: bool,
    job_count: Option<usize>,
    use_cache: bool,
    verbose: bool,
}

impl Default for FFmpegBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FFmpegBuilder {
    /// 创建一个新的构建器实例
    pub fn new() -> Self {
        Self {
            build_dir: None,
            source_dir: None,
            components: HashSet::new(),
            audio_codecs: HashSet::new(),
            video_codecs: HashSet::new(),
            muxer_formats: HashSet::new(),
            enable_hwaccel: true,
            job_count: None,
            use_cache: true,
            verbose: false,
        }
    }

    /// 设置编译输出目录
    pub fn build_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.build_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// 设置源码目录
    pub fn source_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.source_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// 添加一个组件
    pub fn with_component(mut self, component: Component) -> Self {
        self.components.insert(component);
        self
    }

    /// 添加多个组件
    pub fn with_components<I: IntoIterator<Item = Component>>(mut self, components: I) -> Self {
        self.components.extend(components);
        self
    }

    /// 添加一个音频编解码器
    pub fn with_audio_codec(mut self, codec: AudioCodec) -> Self {
        self.audio_codecs.insert(codec);
        if codec == AudioCodec::Opus {
            self.components.insert(Component::Opus);
        }
        self
    }

    /// 添加多个音频编解码器
    pub fn with_audio_codecs<I: IntoIterator<Item = AudioCodec>>(mut self, codecs: I) -> Self {
        for codec in codecs {
            self = self.with_audio_codec(codec);
        }
        self
    }

    /// 添加一个视频编解码器
    pub fn with_video_codec(mut self, codec: VideoCodec) -> Self {
        self.video_codecs.insert(codec);
        if codec == VideoCodec::AV1 {
            self.components.insert(Component::AOM);
        }
        self
    }

    /// 添加多个视频编解码器
    pub fn with_video_codecs<I: IntoIterator<Item = VideoCodec>>(mut self, codecs: I) -> Self {
        for codec in codecs {
            self = self.with_video_codec(codec);
        }
        self
    }

    /// 添加一个封装格式
    pub fn with_muxer_format(mut self, format: MuxerFormat) -> Self {
        self.muxer_formats.insert(format);
        self
    }

    /// 添加多个封装格式
    pub fn with_muxer_formats<I: IntoIterator<Item = MuxerFormat>>(mut self, formats: I) -> Self {
        self.muxer_formats.extend(formats);
        self
    }

    /// 设置是否启用硬件加速
    pub fn enable_hwaccel(mut self, enable: bool) -> Self {
        self.enable_hwaccel = enable;
        self
    }

    /// 设置编译线程数
    pub fn job_count(mut self, count: usize) -> Self {
        self.job_count = Some(count);
        self
    }

    /// 设置是否使用缓存
    pub fn use_cache(mut self, enable: bool) -> Self {
        self.use_cache = enable;
        self
    }

    /// 设置是否启用详细日志
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }

    /// 构建 FFmpeg 编译器
    pub fn build(self) -> FFmpegCompiler {
        // 设置默认目录
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let build_dir = self
            .build_dir
            .unwrap_or_else(|| current_dir.join("ffmpeg_build"));
        let source_dir = self.source_dir.unwrap();

        // 确保至少包含 zlib
        let mut components = self.components;
        components.insert(Component::ZLib);

        FFmpegCompiler::new(
            build_dir,
            source_dir,
            components,
            self.audio_codecs,
            self.video_codecs,
            self.muxer_formats,
            self.enable_hwaccel,
            self.job_count,
            self.use_cache,
            self.verbose,
        )
    }
}
