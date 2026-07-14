/// FFmpeg 组件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component {
    /// Opus 音频编解码器
    Opus,
    /// AV1 视频编解码器（AOM 实现）
    AOM,
    /// zlib 压缩库
    ZLib,
}

/// 音频编解码器
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioCodec {
    /// Opus 音频编解码器
    Opus,
    /// AAC 音频编解码器
    AAC,
    /// MP3 音频编解码器
    MP3,
    /// FLAC 无损音频编解码器
    FLAC,
    /// PCM 音频编解码器
    PCM,
}

/// 视频编解码器
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    /// MPEG-1 Video 解码器（用于旧资源规整）
    MPEG1,
    /// H.264/AVC 视频编解码器
    H264,
    /// H.265/HEVC 视频编解码器
    HEVC,
    /// AV1 视频编解码器
    AV1,
    /// VP9 视频编解码器
    VP9,
}

/// 封装格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MuxerFormat {
    /// Matroska 容器格式（.mkv）
    Matroska,
    /// MP4 容器格式（.mp4）
    MP4,
    /// WebM 容器格式
    WebM,
    /// Ogg 容器格式
    Ogg,
    /// MOV 容器格式（.mov）
    MOV,
    /// WAV 容器格式（.wav）
    WAV,
}
