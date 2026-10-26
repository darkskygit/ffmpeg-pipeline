//! Regression: true resampling (src rate != dst rate) must work end to end.
//! Before the fix, abuffersink `sample_rates` received an 8-byte i64 blob
//! which abuffersink reads as an int list `{rate, 0}`; graph negotiation then
//! picked output rate 0 and aresample failed with
//! "[SWR] Requested output sample rate 0 is invalid" on the first frame.
use ffmpeg_next::ChannelLayout;
use ffmpeg_pipeline::{AudioFrame, AudioSpec, AutoAudioBuffer, Sample, SampleType};

fn push_pull(src_rate: u32, dst_rate: u32) -> usize {
    let layout = ChannelLayout::STEREO;
    let src = AudioSpec::new(layout, Sample::F32(SampleType::Packed), src_rate);
    let dst = AudioSpec::new(layout, Sample::F32(SampleType::Packed), dst_rate);
    let mut buffer = AutoAudioBuffer::new(&src, &dst).expect("graph must validate");

    let mut frame = AudioFrame::new(Sample::F32(SampleType::Packed), 441, layout);
    frame.set_rate(src_rate);
    frame.set_samples(441);
    frame.plane_mut::<f32>(0)[..441].fill(0.25);
    buffer.add_frame(&frame).expect("add frame");
    buffer.flush().expect("flush");

    let mut out = 0usize;
    buffer
        .recv_frames(&mut |f| {
            assert_eq!(f.rate(), dst_rate, "output frame must be at dst rate");
            out += f.samples();
            Ok(())
        })
        .expect("pull frames");
    out
}

#[test]
fn resamples_44k1_to_48k() {
    assert!(push_pull(44100, 48000) > 0);
}

#[test]
fn same_rate_still_works() {
    assert!(push_pull(48000, 48000) > 0);
}

#[test]
fn resamples_mono_s16_22k_to_mono_f32_48k() {
    // decode_audio_pcm shape: decoder spec (mono, s16, 22050) -> contract
    // (mono, f32 packed, 48000); format AND rate mismatch together.
    let layout = ChannelLayout::MONO;
    let src = AudioSpec::new(layout, Sample::I16(SampleType::Packed), 22050);
    let dst = AudioSpec::new(layout, Sample::F32(SampleType::Packed), 48000);
    let mut buffer = AutoAudioBuffer::new(&src, &dst).expect("graph must validate");
    let mut frame = AudioFrame::new(Sample::I16(SampleType::Packed), 2205, layout);
    frame.set_rate(22050);
    frame.set_samples(2205);
    frame.plane_mut::<i16>(0)[..2205].fill(1000);
    buffer.add_frame(&frame).expect("add frame");
    buffer.flush().expect("flush");
    let mut out = 0usize;
    buffer
        .recv_frames(&mut |f| {
            assert_eq!(f.rate(), 48000);
            out += f.samples();
            Ok(())
        })
        .expect("pull frames");
    assert!(out > 0);
}
