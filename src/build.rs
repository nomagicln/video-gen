use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{error::VideoGenError, log::BuildEvent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryOptions {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
}

impl Default for BinaryOptions {
    fn default() -> Self {
        Self {
            ffmpeg: None,
            ffprobe: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodeOptions {
    pub fps: u32,
    pub crf: u8,
    pub preset: String,
    pub audio_bitrate: String,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            fps: 30,
            crf: 20,
            preset: "medium".to_string(),
            audio_bitrate: "192k".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildOptions {
    pub input_dir: PathBuf,
    pub output_path: PathBuf,
    pub work_dir: PathBuf,
    pub keep_temp: bool,
    pub plan: crate::plan::PlanOptions,
    pub encode: EncodeOptions,
    pub binaries: BinaryOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildResult {
    pub output: PathBuf,
    pub bytes: u64,
    pub duration_ms: u64,
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn has_trailing_separator(path: &Path) -> bool {
    let s = path.as_os_str().to_string_lossy();
    s.ends_with('/') || s.ends_with('\\')
}

pub fn resolve_output_path(raw_output: Option<&Path>, input_dir: &Path) -> PathBuf {
    let fallback = PathBuf::from("output").join(format!(
        "{}.mp4",
        input_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("input")
    ));
    let raw = raw_output.unwrap_or(&fallback);
    let resolved = absolute_path(raw);

    let is_dir = match fs::metadata(&resolved) {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => raw_output.is_some() && (has_trailing_separator(raw) || resolved.extension().is_none()),
    };

    if is_dir {
        resolved.join("output.mp4")
    } else {
        resolved
    }
}

pub fn make_run_id(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("build-{seconds}")
}

pub fn build_video<F>(_options: BuildOptions, _on_event: F) -> Result<BuildResult, VideoGenError>
where
    F: FnMut(BuildEvent),
{
    Err(VideoGenError::runtime("not implemented"))
}
