use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::plan::SegmentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogMode {
    Text,
    Quiet,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentStatus {
    Ok,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum BuildEvent {
    Discover {
        units: usize,
        orphans: usize,
    },
    Warn {
        message: String,
    },
    Plan {
        segments: usize,
        total_ms: u64,
        summary: String,
    },
    Segment {
        index: usize,
        total: usize,
        name: String,
        kind: SegmentKind,
        basename: Option<String>,
        duration_ms: u64,
        elapsed_ms: u64,
        status: SegmentStatus,
    },
    Concat {
        output: PathBuf,
        bytes: u64,
        duration_ms: u64,
    },
    Done {
        output: PathBuf,
        bytes: u64,
        duration_ms: u64,
        elapsed_ms: u64,
    },
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / 1024.0 / 1024.0)
    }
}

fn fmt_sec(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}

fn kind_label(kind: SegmentKind) -> &'static str {
    match kind {
        SegmentKind::LeadIn => "lead-in",
        SegmentKind::Unit => "unit",
        SegmentKind::Gap => "gap",
        SegmentKind::Tail => "tail",
    }
}

fn status_label(status: SegmentStatus) -> &'static str {
    match status {
        SegmentStatus::Ok => "ok",
        SegmentStatus::Fail => "fail",
    }
}

pub fn format_event_stdout(mode: LogMode, event: &BuildEvent) -> Option<String> {
    match mode {
        LogMode::Text => format_text_stdout(event),
        LogMode::Quiet => format_quiet_stdout(event),
        LogMode::Json => format_json_stdout(event),
    }
}

pub fn format_event_stderr(mode: LogMode, event: &BuildEvent) -> Option<String> {
    match event {
        BuildEvent::Warn { message } => match mode {
            LogMode::Json => Some(json!({"phase":"warn","message":message}).to_string() + "\n"),
            LogMode::Text | LogMode::Quiet => Some(format!("WARN {message}\n")),
        },
        _ => None,
    }
}

pub fn format_error(mode: LogMode, message: &str, code: i32) -> String {
    match mode {
        LogMode::Json => json!({"phase":"error","code":code,"message":message}).to_string() + "\n",
        LogMode::Text | LogMode::Quiet => format!("[error] {message}\n"),
    }
}

pub fn emit_event(mode: LogMode, event: &BuildEvent) {
    if let Some(line) = format_event_stdout(mode, event) {
        print!("{line}");
    }
    if let Some(line) = format_event_stderr(mode, event) {
        eprint!("{line}");
    }
}

pub fn emit_error(mode: LogMode, message: &str, code: i32) {
    eprint!("{}", format_error(mode, message, code));
}

fn format_text_stdout(event: &BuildEvent) -> Option<String> {
    match event {
        BuildEvent::Discover { units, orphans } => Some(format!(
            "[discover] {units} units ({orphans} orphans skipped)\n"
        )),
        BuildEvent::Warn { .. } => None,
        BuildEvent::Plan {
            segments,
            total_ms,
            summary,
        } => Some(format!(
            "[plan]     {segments} segments ({summary}) total {}\n",
            fmt_sec(*total_ms)
        )),
        BuildEvent::Segment {
            index,
            total,
            name,
            kind,
            basename,
            duration_ms,
            elapsed_ms,
            status,
        } => {
            let idx = format!("[{index:>2}/{total}]");
            let label = if *kind == SegmentKind::Unit {
                format!("unit {}", basename.as_deref().unwrap_or(""))
            } else {
                kind_label(*kind).to_string()
            };
            Some(format!(
                "[build]    {idx} {name:<8} {label:<20} {:.3}s ... {} ({})\n",
                *duration_ms as f64 / 1000.0,
                status_label(*status),
                fmt_sec(*elapsed_ms)
            ))
        }
        BuildEvent::Concat {
            output,
            bytes,
            duration_ms,
        } => Some(format!(
            "[concat]   {} ({}, {})\n",
            output.display(),
            format_bytes(*bytes),
            fmt_sec(*duration_ms)
        )),
        BuildEvent::Done { elapsed_ms, .. } => Some(format!("done in {}\n", fmt_sec(*elapsed_ms))),
    }
}

fn format_quiet_stdout(event: &BuildEvent) -> Option<String> {
    match event {
        BuildEvent::Done {
            output,
            bytes,
            duration_ms,
            ..
        } => Some(format!(
            "done: {} ({}, {})\n",
            output.display(),
            format_bytes(*bytes),
            fmt_sec(*duration_ms)
        )),
        _ => None,
    }
}

fn format_json_stdout(event: &BuildEvent) -> Option<String> {
    match event {
        BuildEvent::Discover { units, orphans } => {
            Some(json!({"phase":"discover","units":units,"orphans":orphans}).to_string() + "\n")
        }
        BuildEvent::Warn { .. } => None,
        BuildEvent::Plan {
            segments, total_ms, ..
        } => {
            Some(json!({"phase":"plan","segments":segments,"total_ms":total_ms}).to_string() + "\n")
        }
        BuildEvent::Segment {
            name,
            kind,
            basename,
            duration_ms,
            elapsed_ms,
            status,
            ..
        } => {
            let mut value = json!({
                "phase": "build",
                "seg": name,
                "kind": kind_label(*kind),
                "duration_ms": duration_ms,
                "status": status_label(*status),
                "ms": elapsed_ms,
            });
            if let Some(basename) = basename {
                value["basename"] = json!(basename);
            }
            Some(value.to_string() + "\n")
        }
        BuildEvent::Concat {
            output,
            bytes,
            duration_ms,
        } => Some(
            json!({
                "phase":"concat",
                "output": output,
                "bytes": bytes,
                "duration_ms": duration_ms
            })
            .to_string()
                + "\n",
        ),
        BuildEvent::Done {
            output,
            bytes,
            duration_ms,
            elapsed_ms,
        } => Some(
            json!({
                "phase":"done",
                "output": output,
                "bytes": bytes,
                "duration_ms": duration_ms,
                "ms": elapsed_ms
            })
            .to_string()
                + "\n",
        ),
    }
}
