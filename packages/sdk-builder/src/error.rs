use thiserror::Error;

/// FFmpeg 编译器错误
#[derive(Error, Debug)]
pub enum FFmpegError {
    #[error("IO 错误: {0}")]
    IO(#[from] std::io::Error),

    #[error("Git 错误: {0}")]
    Git(String),

    #[error("下载错误: {0}")]
    Download(String),

    #[error("文件校验失败: 期望 {expected}, 实际 {actual}")]
    Verification { expected: String, actual: String },

    #[error("构建错误: {0}")]
    Build(String),

    #[error("不支持的平台: {0}")]
    UnsupportedPlatform(String),
}
