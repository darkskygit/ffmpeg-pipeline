mod auto_buffer;
mod resampling;
mod spec;
mod transcode;

use super::*;
use std::convert::{TryFrom, TryInto};

pub use auto_buffer::AutoAudioBuffer;
pub use resampling::Resampler;
pub(crate) use spec::decoder_channel_layout;
pub use spec::AudioSpec;
pub use transcode::transcode_audio_buffer;
