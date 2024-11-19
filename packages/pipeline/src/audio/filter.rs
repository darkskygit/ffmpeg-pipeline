use super::*;
use ffmpeg_next::{
    codec::Capabilities,
    filter::{self, Graph},
};

/// auto re-sample and buffer the audio frame from the decoder to the encoder
///
/// some encode, like opus support variable frame size, the frame decoded from other format
/// may not be the same as the frame size of the encoder, this filter graph will handle the
/// re-sample and frame split.
pub fn audio_buffer<S, D>(src: S, dst: D) -> FFmpegResult<Graph>
where
    S: TryInto<AudioSpec, Error = FFmpegError>,
    D: TryInto<AudioSpec, Error = FFmpegError>,
{
    let src: AudioSpec = src.try_into()?;
    let dst: AudioSpec = dst.try_into()?;

    let mut filter = filter::Graph::new();

    let args = format!(
        "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
        src.time_base,
        src.sample_rate,
        src.format.name(),
        src.channel_layout.bits()
    );

    filter.add(&filter::find("abuffer").unwrap(), "in", &args)?;
    filter.add(&filter::find("abuffersink").unwrap(), "out", "")?;

    {
        let mut out = filter.get("out").unwrap();

        out.set_sample_format(dst.format);
        out.set_channel_layout(dst.channel_layout);
        out.set_sample_rate(dst.sample_rate);
    }

    filter.output("in", 0)?.input("out", 0)?.parse("anull")?;
    filter.validate()?;

    println!("{}", filter.dump());

    if let Some(codec) = dst.codec {
        if !codec
            .capabilities()
            .contains(Capabilities::VARIABLE_FRAME_SIZE)
        {
            filter
                .get("out")
                .unwrap()
                .sink()
                .set_frame_size(dst.frame_size);
        }
    }

    Ok(filter)
}
