use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use video_gen::{
    build_video, BinaryOptions, BuildOptions, EncodeOptions, PlanOptions, VideoGenError,
};

fn has_tool(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run(bin: &str, args: &[String]) {
    let output = Command::new(bin).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{bin} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn make_image(path: &Path, width: u32, height: u32, color: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    run(
        "ffmpeg",
        &[
            "-y".to_string(),
            "-f".to_string(),
            "lavfi".to_string(),
            "-i".to_string(),
            format!("color=c={color}:s={width}x{height}:d=1"),
            "-frames:v".to_string(),
            "1".to_string(),
            path.to_string_lossy().into_owned(),
        ],
    );
}

fn make_silent_wav(path: &Path, duration_sec: f64) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    run(
        "ffmpeg",
        &[
            "-y".to_string(),
            "-f".to_string(),
            "lavfi".to_string(),
            "-i".to_string(),
            "anullsrc=r=44100:cl=stereo".to_string(),
            "-t".to_string(),
            duration_sec.to_string(),
            path.to_string_lossy().into_owned(),
        ],
    );
}

fn probe_duration_sec(path: &Path) -> f64 {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-of",
            "json",
            "-show_entries",
            "format=duration",
            &path.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    data["format"]["duration"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap()
}

fn build_options(work_dir: &Path, input_dir: &Path, output_path: &Path) -> BuildOptions {
    BuildOptions {
        input_dir: input_dir.to_path_buf(),
        output_path: output_path.to_path_buf(),
        work_dir: work_dir.to_path_buf(),
        keep_temp: false,
        plan: PlanOptions {
            lead_in_ms: 1000,
            tail_ms: 1000,
            gap_ms: 500,
        },
        encode: EncodeOptions {
            fps: 30,
            crf: 28,
            preset: "ultrafast".to_string(),
            audio_bitrate: "128k".to_string(),
        },
        binaries: BinaryOptions::default(),
    }
}

#[test]
fn produces_single_mp4_with_expected_duration_and_cleans_temp_dir() {
    if !has_tool("ffmpeg") || !has_tool("ffprobe") {
        eprintln!("skipping e2e test because ffmpeg/ffprobe is unavailable");
        return;
    }

    let work = TempDir::new().unwrap();
    let input = work.path().join("input");
    let output = work.path().join("output/test.mp4");

    make_image(&input.join("01_a.png"), 320, 180, "red");
    make_image(&input.join("02_b.png"), 320, 180, "blue");
    make_silent_wav(&input.join("01_a.wav"), 1.0);
    make_silent_wav(&input.join("02_b.wav"), 1.0);

    let mut events = Vec::new();
    let result = build_video(build_options(work.path(), &input, &output), |event| {
        events.push(event);
    })
    .unwrap();

    assert_eq!(result.output, output);
    assert!(output.exists());
    assert!(result.bytes > 0);

    let duration = probe_duration_sec(&output);
    assert!(duration > 4.3, "duration was {duration}");
    assert!(duration < 4.8, "duration was {duration}");

    let build_root = work.path().join(".video-gen");
    assert!(build_root.exists());
    assert_eq!(fs::read_dir(build_root).unwrap().count(), 0);
    assert!(events.iter().any(|event| matches!(event, video_gen::BuildEvent::Done { .. })));
}

#[test]
fn rejects_mismatched_image_dimensions() {
    if !has_tool("ffmpeg") || !has_tool("ffprobe") {
        eprintln!("skipping e2e test because ffmpeg/ffprobe is unavailable");
        return;
    }

    let work = TempDir::new().unwrap();
    let input = work.path().join("input");
    let output = work.path().join("output/test.mp4");

    make_image(&input.join("01_a.png"), 320, 180, "red");
    make_image(&input.join("02_b.png"), 640, 360, "blue");
    make_silent_wav(&input.join("01_a.wav"), 1.0);
    make_silent_wav(&input.join("02_b.wav"), 1.0);

    let err = build_video(build_options(work.path(), &input, &output), |_| {}).unwrap_err();

    assert!(matches!(err, VideoGenError::User { .. }));
    assert!(err.to_string().contains("image dimensions mismatch"));
}
