use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::{build::EncodeOptions, error::VideoGenError, plan::Segment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Ffmpeg,
    Ffprobe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSize {
    pub width: u64,
    pub height: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub code: i32,
    pub stderr: String,
}

#[derive(Debug, Deserialize)]
struct ProbeData {
    streams: Option<Vec<ProbeStream>>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    duration: Option<String>,
    width: Option<u64>,
    height: Option<u64>,
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn format_seconds(duration_ms: u64) -> String {
    format!("{:.3}", duration_ms as f64 / 1000.0)
}

pub fn parse_audio_duration_ms(json_text: &str) -> Result<u64, VideoGenError> {
    let data: ProbeData = serde_json::from_str(json_text)?;
    let stream = data
        .streams
        .as_ref()
        .and_then(|streams| streams.first())
        .ok_or_else(|| VideoGenError::runtime("ffprobe: no audio stream found"))?;
    let duration = stream
        .duration
        .as_ref()
        .ok_or_else(|| VideoGenError::runtime("ffprobe: missing audio duration"))?;
    let seconds = duration
        .parse::<f64>()
        .map_err(|_| VideoGenError::runtime(format!("ffprobe: invalid audio duration \"{duration}\"")))?;
    if !seconds.is_finite() {
        return Err(VideoGenError::runtime(format!(
            "ffprobe: invalid audio duration \"{duration}\""
        )));
    }
    Ok((seconds * 1000.0).round() as u64)
}

pub fn parse_image_size(json_text: &str) -> Result<ImageSize, VideoGenError> {
    let data: ProbeData = serde_json::from_str(json_text)?;
    let stream = data
        .streams
        .as_ref()
        .and_then(|streams| streams.first())
        .ok_or_else(|| VideoGenError::runtime("ffprobe: missing image dimensions"))?;
    let width = stream
        .width
        .ok_or_else(|| VideoGenError::runtime("ffprobe: missing image dimensions"))?;
    let height = stream
        .height
        .ok_or_else(|| VideoGenError::runtime("ffprobe: missing image dimensions"))?;
    Ok(ImageSize { width, height })
}

pub fn segment_argv(segment: &Segment, output: &Path, opts: &EncodeOptions) -> Vec<String> {
    let mut argv = vec![
        "-y".to_string(),
        "-loop".to_string(),
        "1".to_string(),
        "-framerate".to_string(),
        opts.fps.to_string(),
        "-i".to_string(),
        path_arg(segment.image()),
    ];

    match segment {
        Segment::Unit { audio, .. } => {
            argv.extend([
                "-i".to_string(),
                path_arg(audio),
                "-af".to_string(),
                "aresample=48000,aformat=channel_layouts=stereo".to_string(),
            ]);
        }
        Segment::LeadIn { .. } | Segment::Gap { .. } | Segment::Tail { .. } => {
            argv.extend([
                "-f".to_string(),
                "lavfi".to_string(),
                "-i".to_string(),
                "aevalsrc=0:s=48000:c=stereo".to_string(),
            ]);
        }
    }

    argv.extend([
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        opts.preset.clone(),
        "-crf".to_string(),
        opts.crf.to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        opts.audio_bitrate.clone(),
        "-t".to_string(),
        format_seconds(segment.duration_ms()),
        "-shortest".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        path_arg(output),
    ]);
    argv
}

pub fn concat_argv(concat_list_path: &Path, output_path: &Path) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        path_arg(concat_list_path),
        "-c".to_string(),
        "copy".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        path_arg(output_path),
    ]
}

pub fn concat_list_content(segment_filenames: &[String]) -> String {
    segment_filenames
        .iter()
        .map(|filename| format!("file '{}'", filename.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn env_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Ffmpeg => "VIDEO_GEN_FFMPEG",
        Tool::Ffprobe => "VIDEO_GEN_FFPROBE",
    }
}

fn tool_name(tool: Tool) -> String {
    let name = match tool {
        Tool::Ffmpeg => "ffmpeg",
        Tool::Ffprobe => "ffprobe",
    };
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

pub fn resolve_binary(
    tool: Tool,
    explicit: Option<&Path>,
    exec_dir: Option<&Path>,
) -> Result<PathBuf, VideoGenError> {
    if let Some(explicit) = explicit {
        return Ok(explicit.to_path_buf());
    }

    if let Ok(value) = std::env::var(env_key(tool)) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let exe = tool_name(tool);
    if let Some(exec_dir) = exec_dir {
        let sibling = exec_dir.join(&exe);
        if sibling.exists() {
            return Ok(sibling);
        }
    }

    Ok(PathBuf::from(exe))
}

fn tail_bytes(bytes: &[u8], limit: usize) -> String {
    let start = bytes.len().saturating_sub(limit);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

pub fn check_binary(tool: Tool, path: &Path) -> Result<(), VideoGenError> {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .map_err(|_| {
            VideoGenError::user(format!(
                "{} not found at \"{}\". Install ffmpeg or set {}.",
                match tool {
                    Tool::Ffmpeg => "ffmpeg",
                    Tool::Ffprobe => "ffprobe",
                },
                path.display(),
                env_key(tool)
            ))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(VideoGenError::user(format!(
            "{} -version failed (exit {}): {}",
            match tool {
                Tool::Ffmpeg => "ffmpeg",
                Tool::Ffprobe => "ffprobe",
            },
            output.status.code().unwrap_or(-1),
            tail_bytes(&output.stderr, 200)
        )))
    }
}

fn run_ffprobe(path: &Path, args: &[String]) -> Result<String, VideoGenError> {
    let output = Command::new(path).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(VideoGenError::runtime(format!(
            "ffprobe exit {}: {}",
            output.status.code().unwrap_or(-1),
            tail_bytes(&output.stderr, 500)
        )))
    }
}

pub fn probe_audio_duration_ms(ffprobe: &Path, file: &Path) -> Result<u64, VideoGenError> {
    let json = run_ffprobe(
        ffprobe,
        &[
            "-v".to_string(),
            "error".to_string(),
            "-of".to_string(),
            "json".to_string(),
            "-select_streams".to_string(),
            "a:0".to_string(),
            "-show_entries".to_string(),
            "stream=duration".to_string(),
            path_arg(file),
        ],
    )?;
    parse_audio_duration_ms(&json)
}

pub fn probe_image_size(ffprobe: &Path, file: &Path) -> Result<ImageSize, VideoGenError> {
    let json = run_ffprobe(
        ffprobe,
        &[
            "-v".to_string(),
            "error".to_string(),
            "-of".to_string(),
            "json".to_string(),
            "-select_streams".to_string(),
            "v:0".to_string(),
            "-show_entries".to_string(),
            "stream=width,height".to_string(),
            path_arg(file),
        ],
    )?;
    parse_image_size(&json)
}

pub fn run_ffmpeg(ffmpeg: &Path, argv: &[String]) -> Result<CommandResult, VideoGenError> {
    let output = Command::new(ffmpeg).args(argv).output()?;
    Ok(CommandResult {
        code: output.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
