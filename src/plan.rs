use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::VideoGenError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub basename: String,
    pub image_path: PathBuf,
    pub audio_path: PathBuf,
    pub audio_duration_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOptions {
    pub lead_in_ms: u64,
    pub tail_ms: u64,
    pub gap_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentKind {
    LeadIn,
    Unit,
    Gap,
    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Segment {
    LeadIn { image: PathBuf, duration_ms: u64 },
    Unit {
        image: PathBuf,
        audio: PathBuf,
        duration_ms: u64,
    },
    Gap { image: PathBuf, duration_ms: u64 },
    Tail { image: PathBuf, duration_ms: u64 },
}

impl Segment {
    pub fn kind(&self) -> SegmentKind {
        match self {
            Segment::LeadIn { .. } => SegmentKind::LeadIn,
            Segment::Unit { .. } => SegmentKind::Unit,
            Segment::Gap { .. } => SegmentKind::Gap,
            Segment::Tail { .. } => SegmentKind::Tail,
        }
    }

    pub fn image(&self) -> &PathBuf {
        match self {
            Segment::LeadIn { image, .. }
            | Segment::Unit { image, .. }
            | Segment::Gap { image, .. }
            | Segment::Tail { image, .. } => image,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match self {
            Segment::LeadIn { duration_ms, .. }
            | Segment::Unit { duration_ms, .. }
            | Segment::Gap { duration_ms, .. }
            | Segment::Tail { duration_ms, .. } => *duration_ms,
        }
    }
}

pub fn plan_segments(units: &[Unit], opts: &PlanOptions) -> Result<Vec<Segment>, VideoGenError> {
    if units.is_empty() {
        return Err(VideoGenError::user("plan: requires at least one unit"));
    }

    let mut segments = Vec::new();
    if opts.lead_in_ms > 0 {
        segments.push(Segment::LeadIn {
            image: units[0].image_path.clone(),
            duration_ms: opts.lead_in_ms,
        });
    }

    for (index, unit) in units.iter().enumerate() {
        segments.push(Segment::Unit {
            image: unit.image_path.clone(),
            audio: unit.audio_path.clone(),
            duration_ms: unit.audio_duration_ms,
        });

        if index + 1 < units.len() && opts.gap_ms > 0 {
            segments.push(Segment::Gap {
                image: unit.image_path.clone(),
                duration_ms: opts.gap_ms,
            });
        }
    }

    if opts.tail_ms > 0 {
        let last = units.last().expect("units is not empty");
        segments.push(Segment::Tail {
            image: last.image_path.clone(),
            duration_ms: opts.tail_ms,
        });
    }

    Ok(segments)
}
