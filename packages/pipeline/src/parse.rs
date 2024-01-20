use super::*;
use ffmpeg_next::{
    codec::context::Context, decoder::Video, threading::Config as ThreadConfig,
    util::frame::video::Video as VideoFrame,
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

pub fn parse_video_group(path: &Path, frame_calc: FrameCalculation) -> IoResult<VideoGroups> {
    let mut groups = VideoGroups::default();

    for stream in input(&path)?.streams() {
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
                media::Type::Video => "Video",
                media::Type::Audio => "Audio",
                media::Type::Data => "Data",
                media::Type::Subtitle => "Subtitle",
                media::Type::Attachment => "Attachment",
                media::Type::Unknown => "Unknown",
            }
            .to_string(),
        );

        let mut codec = Context::from_parameters(stream.parameters())?;
        codec.set_threading(ThreadConfig::count(
            std::thread::available_parallelism()?.get(),
        ));
        if codec.medium() == media::Type::Video {
            let mut video = codec.decoder().video()?;
            info.size = info
                .size
                .width(video.width() as isize)
                .height(video.height() as isize);

            let ts = Instant::now();

            if !matches!(frame_calc, FrameCalculation::Skip) {
                info.frames =
                    parse_video_stream_frame_count(&path, info.stream, frame_calc.clone())?;
            }

            if cfg!(not(debug_assertions)) {
                info.cost = ts.elapsed();
            }
        }

        for (key, val) in stream.metadata().iter().filter(|(k, _)| *k != "language") {
            info = info.insert(key.to_string(), val.to_string());
        }

        groups.insert(info.get_title(), info);
    }

    Ok(groups)
}

pub fn parse_video_stream_frame_count(
    path: &Path,
    stream_index: u16,
    frame_calc: FrameCalculation,
) -> IoResult<Option<u64>> {
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

    let mut handler = input(&path)?;
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
    use rayon::prelude::*;
    use std::{
        panic::{catch_unwind, AssertUnwindSafe},
        path::PathBuf,
    };

    fn get_paths() -> Vec<PathBuf> {
        std::fs::read_dir("../../tests/assets")
            .unwrap()
            .filter_map(|p| p.ok().map(|p| p.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default()
                        .ends_with("mkv")
            })
            .collect::<Vec<_>>()
    }

    fn diff_video_groups(file: &Path) -> IoResult<()> {
        let info = parse_video_group(&file, FrameCalculation::Fast)?;
        let info1 = parse_video_group(&file, FrameCalculation::Full)?;
        assert_json_diff::assert_json_matches_no_panic(
            &info,
            &info1,
            assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict),
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(())
    }

    #[test]
    fn test_parse_video_groups() {
        ffmpeg_init().unwrap();

        let paths = get_paths();

        paths.par_iter().enumerate().for_each(|(i, file)| {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                if let Err(e) = diff_video_groups(file) {
                    println!("file {}: {}, error: {:?}", i, file.display(), e);
                }
            })) {
                println!("file {}: {}, crash: {:?}", i, file.display(), e);
            }
        });
    }

    fn check_video_frame_count(file: &Path) -> IoResult<()> {
        let info = parse_video_group(&file, FrameCalculation::Skip)?;
        println!(
            "parse cost: {:?}",
            info.values()
                .fold(std::time::Duration::new(0, 0), |acc, i| acc + i.cost)
        );
        let mut info = info
            .values()
            .filter(|i| i.stream_type == "Video")
            .collect::<Vec<_>>();
        info.sort_by(|a, b| a.stream.cmp(&b.stream));

        for stream in info.iter() {
            assert!(
                parse_video_stream_frame_count(&file, stream.stream, FrameCalculation::Fast)?
                    .is_some()
            );
        }

        Ok(())
    }

    #[test]
    fn test_frame_counting() {
        ffmpeg_init().unwrap();

        let paths = get_paths();

        paths.par_iter().enumerate().for_each(|(i, file)| {
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| {
                if let Err(e) = check_video_frame_count(file) {
                    println!("file {}: {}, error: {:?}", i, file.display(), e);
                }
            })) {
                println!("file {}: {}, crash: {:?}", i, file.display(), e);
            }
        });
    }
}
