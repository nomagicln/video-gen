use std::path::PathBuf;

use video_gen::{plan_segments, PlanOptions, Segment, Unit, VideoGenError};

fn unit(basename: &str, duration_ms: u64) -> Unit {
    Unit {
        basename: basename.to_string(),
        image_path: PathBuf::from(format!("/img/{basename}.jpg")),
        audio_path: PathBuf::from(format!("/aud/{basename}.mp3")),
        audio_duration_ms: duration_ms,
    }
}

#[test]
fn rejects_empty_units() {
    let err = plan_segments(&[], &PlanOptions::default()).unwrap_err();
    assert!(matches!(err, VideoGenError::User { .. }));
    assert!(err.to_string().contains("at least one unit"));
}

#[test]
fn one_unit_without_padding_creates_one_segment() {
    let segments = plan_segments(
        &[unit("a", 1000)],
        &PlanOptions {
            lead_in_ms: 0,
            tail_ms: 0,
            gap_ms: 0,
        },
    )
    .unwrap();

    assert_eq!(
        segments,
        vec![Segment::Unit {
            image: PathBuf::from("/img/a.jpg"),
            audio: PathBuf::from("/aud/a.mp3"),
            duration_ms: 1000,
        }]
    );
}

#[test]
fn skips_zero_duration_lead_in_tail_and_gap() {
    let segments = plan_segments(
        &[unit("a", 1000), unit("b", 2000)],
        &PlanOptions {
            lead_in_ms: 0,
            tail_ms: 0,
            gap_ms: 0,
        },
    )
    .unwrap();

    assert_eq!(
        segments,
        vec![
            Segment::Unit {
                image: PathBuf::from("/img/a.jpg"),
                audio: PathBuf::from("/aud/a.mp3"),
                duration_ms: 1000,
            },
            Segment::Unit {
                image: PathBuf::from("/img/b.jpg"),
                audio: PathBuf::from("/aud/b.mp3"),
                duration_ms: 2000,
            },
        ]
    );
}

#[test]
fn three_units_with_full_padding() {
    let segments = plan_segments(
        &[unit("a", 1000), unit("b", 2000), unit("c", 3000)],
        &PlanOptions {
            lead_in_ms: 2000,
            tail_ms: 1000,
            gap_ms: 500,
        },
    )
    .unwrap();

    assert_eq!(
        segments,
        vec![
            Segment::LeadIn {
                image: PathBuf::from("/img/a.jpg"),
                duration_ms: 2000,
            },
            Segment::Unit {
                image: PathBuf::from("/img/a.jpg"),
                audio: PathBuf::from("/aud/a.mp3"),
                duration_ms: 1000,
            },
            Segment::Gap {
                image: PathBuf::from("/img/a.jpg"),
                duration_ms: 500,
            },
            Segment::Unit {
                image: PathBuf::from("/img/b.jpg"),
                audio: PathBuf::from("/aud/b.mp3"),
                duration_ms: 2000,
            },
            Segment::Gap {
                image: PathBuf::from("/img/b.jpg"),
                duration_ms: 500,
            },
            Segment::Unit {
                image: PathBuf::from("/img/c.jpg"),
                audio: PathBuf::from("/aud/c.mp3"),
                duration_ms: 3000,
            },
            Segment::Tail {
                image: PathBuf::from("/img/c.jpg"),
                duration_ms: 1000,
            },
        ]
    );
}

#[test]
fn lead_in_uses_first_image_and_tail_uses_last_image() {
    let segments = plan_segments(
        &[unit("a", 1000), unit("b", 1000)],
        &PlanOptions {
            lead_in_ms: 500,
            tail_ms: 500,
            gap_ms: 0,
        },
    )
    .unwrap();

    assert_eq!(
        segments.first().unwrap(),
        &Segment::LeadIn {
            image: PathBuf::from("/img/a.jpg"),
            duration_ms: 500,
        }
    );
    assert_eq!(
        segments.last().unwrap(),
        &Segment::Tail {
            image: PathBuf::from("/img/b.jpg"),
            duration_ms: 500,
        }
    );
}

#[test]
fn gap_uses_previous_unit_image() {
    let segments = plan_segments(
        &[unit("a", 1000), unit("b", 1000)],
        &PlanOptions {
            lead_in_ms: 0,
            tail_ms: 0,
            gap_ms: 300,
        },
    )
    .unwrap();

    assert!(segments.contains(&Segment::Gap {
        image: PathBuf::from("/img/a.jpg"),
        duration_ms: 300,
    }));
}
