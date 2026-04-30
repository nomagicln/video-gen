use std::path::PathBuf;

use video_gen::log::{format_error, format_event_stderr, format_event_stdout};
use video_gen::{BuildEvent, LogMode, SegmentKind, SegmentStatus};

#[test]
fn text_mode_writes_discover_line_to_stdout() {
    let line = format_event_stdout(
        LogMode::Text,
        &BuildEvent::Discover {
            units: 8,
            orphans: 3,
        },
    )
    .unwrap();

    assert!(line.contains("[discover]"));
    assert!(line.contains("8 units"));
    assert!(line.contains("3 orphans"));
}

#[test]
fn text_mode_writes_warn_to_stderr() {
    let line = format_event_stderr(
        LogMode::Text,
        &BuildEvent::Warn {
            message: "orphan: 03_a.jpg (no audio)".to_string(),
        },
    )
    .unwrap();

    assert!(line.contains("WARN"));
    assert!(line.contains("orphan: 03_a.jpg"));
}

#[test]
fn quiet_mode_suppresses_progress_events() {
    assert!(format_event_stdout(
        LogMode::Quiet,
        &BuildEvent::Discover {
            units: 8,
            orphans: 3,
        },
    )
    .is_none());
    assert!(format_event_stdout(
        LogMode::Quiet,
        &BuildEvent::Segment {
            index: 1,
            total: 11,
            name: "seg_001".to_string(),
            kind: SegmentKind::LeadIn,
            basename: None,
            duration_ms: 2000,
            elapsed_ms: 400,
            status: SegmentStatus::Ok,
        },
    )
    .is_none());
}

#[test]
fn quiet_mode_still_writes_done_line() {
    let line = format_event_stdout(
        LogMode::Quiet,
        &BuildEvent::Done {
            output: PathBuf::from("output/v.mp4"),
            bytes: 100,
            duration_ms: 1000,
            elapsed_ms: 5000,
        },
    )
    .unwrap();

    assert!(line.contains("output/v.mp4"));
    assert!(line.contains("done:"));
}

#[test]
fn json_mode_emits_discover_object_on_stdout() {
    let line = format_event_stdout(
        LogMode::Json,
        &BuildEvent::Discover {
            units: 8,
            orphans: 3,
        },
    )
    .unwrap();

    let data: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(
        data,
        serde_json::json!({"phase":"discover","units":8,"orphans":3})
    );
}

#[test]
fn errors_render_as_text_or_json() {
    assert!(format_error(LogMode::Text, "image dimensions mismatch", 2).contains("[error]"));

    let data: serde_json::Value =
        serde_json::from_str(format_error(LogMode::Json, "boom", 2).trim()).unwrap();
    assert_eq!(
        data,
        serde_json::json!({"phase":"error","code":2,"message":"boom"})
    );
}
