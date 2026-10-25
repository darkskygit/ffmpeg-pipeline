use super::*;
use ffmpeg_next::{channel_layout::ChannelLayout, Rational};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpusEncodeOptions {
    pub application: OpusApplication,
    pub complexity: u8,
    pub frame_duration_ms: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpusApplication {
    Audio,
    Voip,
    LowDelay,
}

impl OpusApplication {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Voip => "voip",
            Self::LowDelay => "lowdelay",
        }
    }
}

pub enum EncodeParams {
    Audio {
        bitrate: usize,
        channel_layout: ChannelLayout,
        compression: Option<usize>,
        global_header: bool,
        rate: i32,
        time_base: Rational,
        vbr: bool,
        opus: Option<OpusEncodeOptions>,
    },
    Video {
        time_base: Rational,
        global_header: bool,
    },
}

impl EncodeParams {
    pub fn audio(channel_layout: ChannelLayout, rate: u32) -> Self {
        Self::Audio {
            bitrate: 128 * 1024,
            channel_layout,
            compression: None,
            global_header: false,
            rate: rate as i32,
            time_base: Rational::new(1, rate as i32),
            vbr: false,
            opus: None,
        }
    }

    pub fn time_base(&self) -> Rational {
        match self {
            Self::Audio { time_base, .. } | Self::Video { time_base, .. } => *time_base,
        }
    }

    pub fn with_bitrate(self, bitrate: usize) -> Self {
        match self {
            Self::Audio {
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
                opus,
                ..
            } => Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
                opus,
            },
            value => value,
        }
    }

    pub fn with_compression(self, compression: Option<usize>) -> Self {
        match self {
            Self::Audio {
                bitrate,
                channel_layout,
                global_header,
                rate,
                time_base,
                vbr,
                opus,
                ..
            } => Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
                opus,
            },
            value => value,
        }
    }

    pub fn with_vbr(self, vbr: bool) -> Self {
        match self {
            Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                opus,
                ..
            } => Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
                opus,
            },
            value => value,
        }
    }

    pub fn with_opus_options(self, opus: OpusEncodeOptions) -> Self {
        match self {
            Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
                ..
            } => Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
                opus: Some(opus),
            },
            value => value,
        }
    }
}

impl Default for EncodeParams {
    fn default() -> Self {
        Self::Audio {
            bitrate: 128 * 1024,
            channel_layout: ChannelLayout::STEREO,
            compression: None,
            global_header: false,
            rate: 44100,
            time_base: Rational::new(1, 44100),
            vbr: false,
            opus: None,
        }
    }
}

impl From<&Decoder<'_>> for EncodeParams {
    fn from(decoder: &Decoder<'_>) -> Self {
        match decoder.get_decoder() {
            StreamDecoder::Audio(decoder) => Self::Audio {
                bitrate: decoder.bit_rate(),
                rate: decoder.rate() as i32,
                channel_layout: decoder_channel_layout(decoder),
                compression: None,
                time_base: decoder.time_base(),
                global_header: false,
                vbr: false,
                opus: None,
            },
            StreamDecoder::Video(decoder) => Self::Video {
                time_base: decoder.time_base(),
                global_header: false,
            },
        }
    }
}
