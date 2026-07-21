use super::*;
use ffmpeg_next::{
    codec::context::Context, decoder::Video, media::Type as MediaType,
    threading::Config as ThreadConfig, util::frame::video::Video as VideoFrame, Stream,
};

fn adjust_precision_of_ratio(numerator: f64, denominator: f64) -> f64 {
    let ratio = numerator / denominator;

    match (ratio * 100.0).round() as u64 {
        0 => (ratio * 10000.0).round() / 10000.0,
        v if v % 100 != 0 => (ratio * 100.0).round() / 100.0,
        v if v % (100 * 1000) != 0 => ratio.round(),
        _ => (ratio / 1000.0).round() * 1000.0,
    }
}

pub fn parse_stream_info(stream: &Stream) -> FFmpegResult<VideoInfo> {
    let mut info = VideoInfo::default();

    info = info.stream(stream.index() as u16);

    let rate = stream.avg_frame_rate();
    let fps = adjust_precision_of_ratio(rate.numerator().into(), rate.denominator().into());
    if fps.is_normal() && fps.is_sign_positive() {
        info.fps = Some(fps);
    }

    let parameters = stream.parameters();
    info.format = match parameters.id().name() {
        "h264" => StreamFormat::H264,
        "hevc" => StreamFormat::HEVC,
        "av1" => StreamFormat::AV1,
        format @ "png" => StreamFormat::Other(format.into()),
        _ => StreamFormat::Other("Unknown".into()),
    };
    info = info.stream_type(
        match parameters.medium() {
            MediaType::Video => "Video",
            MediaType::Audio => "Audio",
            MediaType::Data => "Data",
            MediaType::Subtitle => "Subtitle",
            MediaType::Attachment => "Attachment",
            MediaType::Unknown => "Unknown",
        }
        .to_string(),
    );

    let mut codec = Context::from_parameters(stream.parameters())?;
    codec.set_threading(ThreadConfig::count(1));
    if codec.medium() == MediaType::Video {
        let video = codec.decoder().video()?;
        info.size = info
            .size
            .width(video.width() as isize)
            .height(video.height() as isize);
        info.pixel = video.format();
    }

    for (key, val) in stream.metadata().iter().filter(|(k, _)| *k != "language") {
        info = info.insert(key.to_string(), val.to_string());
    }

    Ok(info)
}

pub fn parse_video_group(path: &Path, frame_calc: FrameCalculation) -> FFmpegResult<VideoGroups> {
    let mut groups = VideoGroups::default();

    for stream in input_file(path)?.streams() {
        let mut info = parse_stream_info(&stream)?;

        if info.stream_type == "Video" {
            let ts = Instant::now();

            if !matches!(frame_calc, FrameCalculation::Skip) {
                info.frames = parse_video_stream_frame_count(path, info.stream, frame_calc)?;
            }

            if cfg!(not(debug_assertions)) {
                info.cost = ts.elapsed();
            }
        }

        groups.insert(info.get_title(), info);
    }

    Ok(groups)
}

pub fn parse_video_stream_frame_count(
    path: &Path,
    stream_index: u16,
    frame_calc: FrameCalculation,
) -> FFmpegResult<Option<u64>> {
    if matches!(frame_calc, FrameCalculation::Skip) {
        return Ok(None);
    }
    let mut frame_count = 0;
    let mut calc_frames = |decoder: &mut Video| match frame_calc {
        FrameCalculation::Skip => unreachable!(),
        FrameCalculation::Fast => {
            frame_count += 1;
        }
        FrameCalculation::Full => {
            let mut decoded = VideoFrame::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                frame_count += 1;
            }
        }
    };

    let mut handler = input_file(path)?;
    if let Some(stream) = handler.stream(stream_index as usize) {
        let codec = Context::from_parameters(stream.parameters())?;
        let mut video = codec.decoder().video()?;

        for (stream, packet) in handler.packets() {
            if stream.index() != stream_index as usize {
                continue;
            }

            if matches!(frame_calc, FrameCalculation::Full) {
                video.send_packet(&packet)?;
            }

            calc_frames(&mut video);
        }
        if matches!(frame_calc, FrameCalculation::Full) {
            video.send_eof()?;
            calc_frames(&mut video);
        }

        Ok(Some(frame_count))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_video_groups() {
        assert_eq!(adjust_precision_of_ratio(30_000.0, 1_001.0), 29.97);
        assert_eq!(adjust_precision_of_ratio(24.0, 1.0), 24.0);
    }

    #[test]
    fn test_frame_counting() {
        initialize(log::Level::Error).unwrap();
        let path =
            std::env::temp_dir().join(format!("ffmpeg-pipeline-parse-{}.ivf", std::process::id()));
        std::fs::write(&path, crate::tests::encoded_ivf(3)).unwrap();
        let groups = parse_video_group(&path, FrameCalculation::Full).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(groups.len(), 1);
        let stream = groups.values().next().unwrap();
        assert_eq!(stream.stream_type, "Video");
        assert_eq!(stream.frames, Some(3));
    }
}
