use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
