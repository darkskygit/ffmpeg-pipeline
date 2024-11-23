use super::*;
use ffmpeg_next::{
    codec::Capabilities,
    filter::{self, Context, Graph},
};

/// auto re-sample and buffer the audio frame from the decoder to the encoder
///
/// some encode, like opus support variable frame size, the frame decoded from other format
/// may not be the same as the frame size of the encoder, this filter graph will handle the
/// re-sample and frame split.
pub struct AutoAudioBuffer {
    graph: Graph,
    input: Context,
    output: Context,
}

impl AutoAudioBuffer {
    pub fn new<S, D>(src: S, dst: D) -> FFmpegResult<Self>
    where
        S: TryInto<AudioSpec, Error = FFmpegError>,
        D: TryInto<AudioSpec, Error = FFmpegError>,
    {
        let src: AudioSpec = src.try_into()?;
        let dst: AudioSpec = dst.try_into()?;

        let mut graph = Graph::new();

        graph.add(
            &filter::find("abuffer").unwrap(),
            "in",
            &format!(
                "time_base={}:sample_rate={}:sample_fmt={}:channel_layout=0x{:x}",
                src.time_base,
                src.sample_rate,
                src.format.name(),
                src.channel_layout.bits()
            ),
        )?;
        graph.add(&filter::find("abuffersink").unwrap(), "out", "")?;

        {
            let mut out = graph.get("out").unwrap();

            out.set_sample_format(dst.format);
            out.set_channel_layout(dst.channel_layout);
            out.set_sample_rate(dst.sample_rate);
        }

        graph.output("in", 0)?.input("out", 0)?.parse("anull")?;
        graph.validate()?;

        if let Some(codec) = dst.codec {
            if !codec
                .capabilities()
                .contains(Capabilities::VARIABLE_FRAME_SIZE)
            {
                graph
                    .get("out")
                    .unwrap()
                    .sink()
                    .set_frame_size(dst.frame_size);
            }
        }

        let input = graph.get("in").unwrap();
        let output = graph.get("out").unwrap();

        Ok(Self {
            graph,
            input,
            output,
        })
    }

    pub fn add_frame(&mut self, frame: &AudioFrame) -> FFmpegResult {
        self.input.source().add(frame)?;
        Ok(())
    }

    pub fn recv_frames<F>(&mut self, cb: &mut F) -> FFmpegResult
    where
        F: FnMut(AudioFrame) -> FFmpegResult,
    {
        let mut sink = self.output.sink();
        let mut filtered = AudioFrame::empty();
        while sink.frame(&mut filtered).is_ok() {
            cb(filtered.clone())?;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> FFmpegResult {
        self.input.source().flush()?;
        Ok(())
    }
}
