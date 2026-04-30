use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use video_gen::build::resolve_output_path;

fn input_dir() -> &'static Path {
    Path::new("/tmp/some-input")
}

#[test]
fn uses_default_output_named_after_input_dir() {
    let output = resolve_output_path(None, input_dir());
    assert_eq!(
        output,
        std::env::current_dir()
            .unwrap()
            .join("output")
            .join("some-input.mp4")
    );
}

#[test]
fn returns_given_file_path_unchanged() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("movie.mp4");

    let output = resolve_output_path(Some(&file), input_dir());

    assert_eq!(output, file);
}

#[test]
fn appends_output_mp4_for_existing_directory() {
    let dir = TempDir::new().unwrap();

    let output = resolve_output_path(Some(dir.path()), input_dir());

    assert_eq!(output, dir.path().join("output.mp4"));
}

#[test]
fn appends_output_mp4_when_path_ends_with_separator() {
    let dir = TempDir::new().unwrap();
    let ghost = PathBuf::from(format!(
        "{}{}",
        dir.path().join("not-yet-created").display(),
        std::path::MAIN_SEPARATOR
    ));

    let output = resolve_output_path(Some(&ghost), input_dir());

    assert_eq!(output, dir.path().join("not-yet-created").join("output.mp4"));
}

#[test]
fn treats_absent_extensionless_path_as_directory() {
    let dir = TempDir::new().unwrap();
    let ghost = dir.path().join("not-yet-created");

    let output = resolve_output_path(Some(&ghost), input_dir());

    assert_eq!(output, ghost.join("output.mp4"));
}

#[test]
fn keeps_absent_path_with_extension_as_file_path() {
    let dir = TempDir::new().unwrap();
    let ghost = dir.path().join("unborn").join("movie.mp4");

    let output = resolve_output_path(Some(&ghost), input_dir());

    assert_eq!(output, ghost);
}

#[test]
fn keeps_existing_file_path_unchanged() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("existing.mp4");
    fs::write(&file, "").unwrap();

    let output = resolve_output_path(Some(&file), input_dir());

    assert_eq!(output, file);
}
