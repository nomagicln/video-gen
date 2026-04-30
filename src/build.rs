use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::discover::discover;
use crate::error::VideoGenError;
use crate::ffmpeg::{
    check_binary, concat_argv, concat_list_content, probe_audio_duration_ms, probe_image_size,
    resolve_binary, run_ffmpeg, segment_argv, Tool,
};
use crate::log::{BuildEvent, SegmentStatus};
use crate::plan::{plan_segments, Segment, SegmentKind, Unit};

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
        Err(_) => {
            raw_output.is_some() && (has_trailing_separator(raw) || resolved.extension().is_none())
        }
    };

    if is_dir {
        resolved.join("output.mp4")
    } else {
        resolved
    }
}

pub fn make_run_id(now: SystemTime) -> String {
    let millis = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("build-{millis}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDir {
    base: PathBuf,
}

impl BuildDir {
    pub fn new(root_cwd: &Path, run_id: &str) -> Self {
        Self {
            base: root_cwd.join(".video-gen").join(run_id),
        }
    }

    pub fn init(&self) -> Result<(), VideoGenError> {
        fs::create_dir_all(&self.base)?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.base
    }

    pub fn seg_path(&self, index: usize) -> PathBuf {
        self.base.join(format!("seg_{index:03}.mp4"))
    }

    pub fn concat_list_path(&self) -> PathBuf {
        self.base.join("concat.txt")
    }

    pub fn cleanup(&self) -> Result<(), VideoGenError> {
        fs::remove_dir_all(&self.base)?;
        Ok(())
    }
}

fn last_lines(text: &str, n: usize) -> String {
    text.trim()
        .lines()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

fn segment_kind_label(kind: SegmentKind) -> &'static str {
    match kind {
        SegmentKind::LeadIn => "lead-in",
        SegmentKind::Unit => "unit",
        SegmentKind::Gap => "gap",
        SegmentKind::Tail => "tail",
    }
}

fn summarize(segments: &[Segment]) -> String {
    let mut lead_in = None;
    let mut units = 0usize;
    let mut gaps = 0usize;
    let mut gap_ms = 0u64;
    let mut tail = None;

    for segment in segments {
        match segment {
            Segment::LeadIn { duration_ms, .. } => lead_in = Some(*duration_ms),
            Segment::Unit { .. } => units += 1,
            Segment::Gap { duration_ms, .. } => {
                gaps += 1;
                gap_ms = *duration_ms;
            }
            Segment::Tail { duration_ms, .. } => tail = Some(*duration_ms),
        }
    }

    let mut parts = Vec::new();
    if let Some(duration_ms) = lead_in {
        parts.push(format!("lead-in {:.1}s", duration_ms as f64 / 1000.0));
    }
    parts.push(format!("{units}x unit"));
    if gaps > 0 {
        parts.push(format!("{gaps}x gap {:.1}s", gap_ms as f64 / 1000.0));
    }
    if let Some(duration_ms) = tail {
        parts.push(format!("tail {:.1}s", duration_ms as f64 / 1000.0));
    }
    parts.join(", ")
}

fn current_exec_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn segment_basename(segment: &Segment, units: &[Unit]) -> Option<String> {
    let Segment::Unit { image, .. } = segment else {
        return None;
    };
    units
        .iter()
        .find(|unit| &unit.image_path == image)
        .map(|unit| unit.basename.clone())
}

pub fn build_video<F>(options: BuildOptions, mut on_event: F) -> Result<BuildResult, VideoGenError>
where
    F: FnMut(BuildEvent),
{
    let started = Instant::now();
    let exec_dir = current_exec_dir();
    let ffmpeg = resolve_binary(
        Tool::Ffmpeg,
        options.binaries.ffmpeg.as_deref(),
        exec_dir.as_deref(),
    )?;
    let ffprobe = resolve_binary(
        Tool::Ffprobe,
        options.binaries.ffprobe.as_deref(),
        exec_dir.as_deref(),
    )?;

    check_binary(Tool::Ffmpeg, &ffmpeg)?;
    check_binary(Tool::Ffprobe, &ffprobe)?;

    let discovered = discover(&options.input_dir)?;
    for orphan in &discovered.orphans {
        let missing = match orphan.kind {
            crate::discover::OrphanKind::Image => "audio",
            crate::discover::OrphanKind::Audio => "image",
        };
        let name = orphan
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(orphan.basename.as_str());
        on_event(BuildEvent::Warn {
            message: format!("orphan: {name} (no {missing})"),
        });
    }
    on_event(BuildEvent::Discover {
        units: discovered.pairs.len(),
        orphans: discovered.orphans.len(),
    });

    let mut image_sizes = Vec::with_capacity(discovered.pairs.len());
    for pair in &discovered.pairs {
        image_sizes.push(probe_image_size(&ffprobe, &pair.image)?);
    }
    if let Some(reference) = image_sizes.first() {
        for (index, size) in image_sizes.iter().enumerate().skip(1) {
            if size.width != reference.width || size.height != reference.height {
                return Err(VideoGenError::user(format!(
                    "image dimensions mismatch:\n  expected {}x{} (from {})\n  got      {}x{}  in    {}",
                    reference.width,
                    reference.height,
                    discovered.pairs[0]
                        .image
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("<unknown>"),
                    size.width,
                    size.height,
                    discovered.pairs[index]
                        .image
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("<unknown>")
                )));
            }
        }
    }

    let mut units = Vec::with_capacity(discovered.pairs.len());
    for pair in &discovered.pairs {
        let audio_duration_ms = probe_audio_duration_ms(&ffprobe, &pair.audio)?;
        units.push(Unit {
            basename: pair.basename.clone(),
            image_path: pair.image.clone(),
            audio_path: pair.audio.clone(),
            audio_duration_ms,
        });
    }

    let segments = plan_segments(&units, &options.plan)?;
    let total_ms = segments.iter().map(Segment::duration_ms).sum::<u64>();
    on_event(BuildEvent::Plan {
        segments: segments.len(),
        total_ms,
        summary: summarize(&segments),
    });

    let run_id = make_run_id(SystemTime::now());
    let build_dir = BuildDir::new(&options.work_dir, &run_id);
    build_dir.init()?;

    let mut success = false;
    let result = (|| {
        let mut segment_names = Vec::with_capacity(segments.len());
        for (index, segment) in segments.iter().enumerate() {
            let segment_number = index + 1;
            let segment_path = build_dir.seg_path(segment_number);
            let segment_name = segment_path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("seg")
                .to_string();
            let segment_started = Instant::now();
            let argv = segment_argv(segment, &segment_path, &options.encode);
            let run = run_ffmpeg(&ffmpeg, &argv)?;
            let elapsed_ms = segment_started.elapsed().as_millis() as u64;
            let basename = segment_basename(segment, &units);

            if run.code != 0 {
                on_event(BuildEvent::Segment {
                    index: segment_number,
                    total: segments.len(),
                    name: segment_name.clone(),
                    kind: segment.kind(),
                    basename,
                    duration_ms: segment.duration_ms(),
                    elapsed_ms,
                    status: SegmentStatus::Fail,
                });
                return Err(VideoGenError::runtime(format!(
                    "segment {} ({}) failed (ffmpeg exit {}):\n{}",
                    segment_name,
                    segment_kind_label(segment.kind()),
                    run.code,
                    last_lines(&run.stderr, 20)
                )));
            }

            on_event(BuildEvent::Segment {
                index: segment_number,
                total: segments.len(),
                name: segment_name,
                kind: segment.kind(),
                basename,
                duration_ms: segment.duration_ms(),
                elapsed_ms,
                status: SegmentStatus::Ok,
            });
            segment_names.push(
                segment_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .to_string(),
            );
        }

        let concat_list_path = build_dir.concat_list_path();
        fs::write(&concat_list_path, concat_list_content(&segment_names))?;
        if let Some(parent) = options.output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let concat = run_ffmpeg(
            &ffmpeg,
            &concat_argv(&concat_list_path, &options.output_path),
        )?;
        if concat.code != 0 {
            return Err(VideoGenError::runtime(format!(
                "concat failed (ffmpeg exit {}):\n{}",
                concat.code,
                last_lines(&concat.stderr, 20)
            )));
        }

        let bytes = fs::metadata(&options.output_path)?.len();
        on_event(BuildEvent::Concat {
            output: options.output_path.clone(),
            bytes,
            duration_ms: total_ms,
        });
        on_event(BuildEvent::Done {
            output: options.output_path.clone(),
            bytes,
            duration_ms: total_ms,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });

        success = true;
        Ok(BuildResult {
            output: options.output_path.clone(),
            bytes,
            duration_ms: total_ms,
        })
    })();

    if success && !options.keep_temp {
        build_dir.cleanup()?;
    }

    result
}
