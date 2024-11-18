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
pub fn audio_buffer(decoder: &Decoder, encoder: &Encoder) -> FFmpegResult<Graph> {
    let (StreamDecoder::Audio(decoder), StreamEncoder::Audio(encoder)) =
        (decoder.get_decoder(), encoder.get_encoder())
    else {
        return Err(FFmpegError::InvalidStreamType("Video".into()));
    };
    let mut filter = filter::Graph::new();

    let args = format!(
        "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
        decoder.time_base(),
        decoder.rate(),
        decoder.format().name(),
        decoder.channel_layout().bits()
    );

    filter.add(&filter::find("abuffer").unwrap(), "in", &args)?;
    filter.add(&filter::find("abuffersink").unwrap(), "out", "")?;

    {
        let mut out = filter.get("out").unwrap();

        out.set_sample_format(encoder.format());
        out.set_channel_layout(encoder.channel_layout());
        out.set_sample_rate(encoder.rate());
    }

    filter.output("in", 0)?.input("out", 0)?.parse("anull")?;
    filter.validate()?;

    println!("{}", filter.dump());

    if let Some(codec) = encoder.codec() {
        if !codec
            .capabilities()
            .contains(Capabilities::VARIABLE_FRAME_SIZE)
        {
            filter
                .get("out")
                .unwrap()
                .sink()
                .set_frame_size(encoder.frame_size());
        }
    }

    Ok(filter)
}
