pub mod build;
pub mod discover;
pub mod error;
pub mod ffmpeg;
pub mod log;
pub mod plan;

pub use build::{build_video, BinaryOptions, BuildOptions, BuildResult, EncodeOptions};
pub use discover::{discover, DiscoverResult, Orphan, OrphanKind, Pair};
pub use error::{ErrorKind, ErrorPayload, VideoGenError};
pub use log::{BuildEvent, LogMode, SegmentStatus};
pub use plan::{plan_segments, PlanOptions, Segment, SegmentKind, Unit};
