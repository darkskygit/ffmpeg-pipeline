use super::*;
use ffmpeg_next::{channel_layout::ChannelLayout, Rational};

pub enum EncodeParams {
    Audio {
        bitrate: usize,
        channel_layout: ChannelLayout,
        compression: Option<usize>,
        global_header: bool,
        rate: i32,
        time_base: Rational,
        vbr: bool,
    },
    Video {
        time_base: Rational,
        global_header: bool,
    },
}

impl EncodeParams {
    pub fn time_base(&self) -> Rational {
        match self {
            EncodeParams::Audio { time_base, .. } => *time_base,
            EncodeParams::Video { time_base, .. } => *time_base,
        }
    }

    pub fn with_bitrate(self, bitrate: usize) -> Self {
        if let Self::Audio {
            channel_layout,
            compression,
            global_header,
            rate,
            time_base,
            vbr,
            ..
        } = self
        {
            Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
            }
        } else {
            self
        }
    }

    pub fn with_compression(self, compression: Option<usize>) -> Self {
        if let Self::Audio {
            bitrate,
            channel_layout,
            global_header,
            rate,
            time_base,
            vbr,
            ..
        } = self
        {
            Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
            }
        } else {
            self
        }
    }

    pub fn with_vbr(self, vbr: bool) -> Self {
        if let Self::Audio {
            bitrate,
            channel_layout,
            compression,
            global_header,
            rate,
            time_base,
            ..
        } = self
        {
            Self::Audio {
                bitrate,
                channel_layout,
                compression,
                global_header,
                rate,
                time_base,
                vbr,
            }
        } else {
            self
        }
    }
}

impl Default for EncodeParams {
    fn default() -> Self {
        EncodeParams::Audio {
            bitrate: 128 * 1024,
            channel_layout: ChannelLayout::STEREO,
            compression: None,
            global_header: false,
            rate: 44100,
            time_base: Rational::new(1, 44100),
            vbr: false,
        }
    }
}

impl From<&Decoder<'_>> for EncodeParams {
    fn from(decoder: &Decoder<'_>) -> Self {
        match decoder.get_decoder() {
            StreamDecoder::Audio(decoder) => EncodeParams::Audio {
                bitrate: decoder.bit_rate(),
                rate: decoder.rate() as i32,
                channel_layout: decoder_channel_layout(decoder),
                compression: None,
                time_base: decoder.time_base(),
                global_header: false,
                vbr: false,
            },
            StreamDecoder::Video(decoder) => EncodeParams::Video {
                time_base: decoder.time_base(),
                global_header: false,
            },
        }
    }
}
