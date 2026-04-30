use std::path::Path;
use std::path::PathBuf;

use video_gen::ffmpeg::{concat_argv, concat_list_content, segment_argv};
use video_gen::{EncodeOptions, Segment};

fn opts() -> EncodeOptions {
    EncodeOptions {
        fps: 30,
        crf: 20,
        preset: "medium".to_string(),
        audio_bitrate: "192k".to_string(),
    }
}

#[test]
fn builds_unit_segment_with_audio_input() {
    let argv = segment_argv(
        &Segment::Unit {
            image: PathBuf::from("/i/a.jpg"),
            audio: PathBuf::from("/a/a.mp3"),
            duration_ms: 4217,
        },
        Path::new("/tmp/seg_002.mp4"),
        &opts(),
    );

    assert_eq!(
        argv,
        vec![
            "-y",
            "-loop",
            "1",
            "-framerate",
            "30",
            "-i",
            "/i/a.jpg",
            "-i",
            "/a/a.mp3",
            "-af",
            "aresample=48000,aformat=channel_layouts=stereo",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-t",
            "4.217",
            "-shortest",
            "-movflags",
            "+faststart",
            "/tmp/seg_002.mp4",
        ]
    );
}

#[test]
fn builds_silent_segment_with_aevalsrc() {
    let argv = segment_argv(
        &Segment::LeadIn {
            image: PathBuf::from("/i/a.jpg"),
            duration_ms: 2000,
        },
        Path::new("/tmp/seg_001.mp4"),
        &opts(),
    );

    assert_eq!(
        argv,
        vec![
            "-y",
            "-loop",
            "1",
            "-framerate",
            "30",
            "-i",
            "/i/a.jpg",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0:s=48000:c=stereo",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-t",
            "2.000",
            "-shortest",
            "-movflags",
            "+faststart",
            "/tmp/seg_001.mp4",
        ]
    );
}

#[test]
fn formats_fractional_seconds_to_three_decimals() {
    let argv = segment_argv(
        &Segment::Gap {
            image: PathBuf::from("/i/x.jpg"),
            duration_ms: 500,
        },
        Path::new("/tmp/seg.mp4"),
        &opts(),
    );

    assert!(argv.contains(&"0.500".to_string()));
}

#[test]
fn respects_custom_encode_options() {
    let argv = segment_argv(
        &Segment::Gap {
            image: PathBuf::from("/i/x.jpg"),
            duration_ms: 500,
        },
        Path::new("/tmp/seg.mp4"),
        &EncodeOptions {
            fps: 25,
            crf: 18,
            preset: "slower".to_string(),
            audio_bitrate: "256k".to_string(),
        },
    );

    assert!(argv.contains(&"25".to_string()));
    assert!(argv.contains(&"18".to_string()));
    assert!(argv.contains(&"slower".to_string()));
    assert!(argv.contains(&"256k".to_string()));
}

#[test]
fn concat_uses_demuxer_with_copy_codec() {
    let argv = concat_argv(Path::new("/tmp/concat.txt"), Path::new("/out/v.mp4"));

    assert_eq!(
        argv,
        vec![
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            "/tmp/concat.txt",
            "-c",
            "copy",
            "-movflags",
            "+faststart",
            "/out/v.mp4",
        ]
    );
}

#[test]
fn concat_list_emits_one_file_line_per_segment() {
    assert_eq!(
        concat_list_content(&["seg_001.mp4".to_string(), "seg_002.mp4".to_string()]),
        "file 'seg_001.mp4'\nfile 'seg_002.mp4'\n"
    );
}

#[test]
fn concat_list_escapes_single_quotes() {
    assert_eq!(
        concat_list_content(&["seg_'weird'.mp4".to_string()]),
        "file 'seg_'\\''weird'\\''.mp4'\n"
    );
}
