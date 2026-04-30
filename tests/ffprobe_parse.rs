use video_gen::ffmpeg::{parse_audio_duration_ms, parse_image_size, ImageSize};

#[test]
fn reads_audio_duration_from_first_stream() {
    let json = r#"{"streams":[{"duration":"4.217333"}]}"#;

    assert_eq!(parse_audio_duration_ms(json).unwrap(), 4217);
}

#[test]
fn rounds_audio_duration_to_nearest_ms() {
    let json = r#"{"streams":[{"duration":"1.0009"}]}"#;

    assert_eq!(parse_audio_duration_ms(json).unwrap(), 1001);
}

#[test]
fn rejects_missing_audio_stream() {
    let err = parse_audio_duration_ms(r#"{"streams":[]}"#).unwrap_err();

    assert!(err.to_string().contains("no audio stream"));
}

#[test]
fn rejects_missing_audio_duration() {
    let err = parse_audio_duration_ms(r#"{"streams":[{}]}"#).unwrap_err();

    assert!(err.to_string().contains("duration"));
}

#[test]
fn rejects_non_finite_audio_duration() {
    let err = parse_audio_duration_ms(r#"{"streams":[{"duration":"N/A"}]}"#).unwrap_err();

    assert!(err.to_string().contains("duration"));
}

#[test]
fn reads_image_size_from_first_stream() {
    let json = r#"{"streams":[{"width":1920,"height":1080}]}"#;

    assert_eq!(
        parse_image_size(json).unwrap(),
        ImageSize {
            width: 1920,
            height: 1080,
        }
    );
}

#[test]
fn rejects_missing_image_dimensions() {
    let err = parse_image_size(r#"{"streams":[{}]}"#).unwrap_err();

    assert!(err.to_string().contains("dimensions"));
}
