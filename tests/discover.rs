use std::fs;
use std::path::Path;

use tempfile::TempDir;
use video_gen::{discover, Orphan, OrphanKind};

fn touch(path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

#[test]
fn pairs_by_basename_in_flat_input_dir() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));
    touch(root.path().join("01_a.mp3"));
    touch(root.path().join("02_b.png"));
    touch(root.path().join("02_b.wav"));

    let result = discover(root.path()).unwrap();

    assert_eq!(
        result
            .pairs
            .iter()
            .map(|pair| pair.basename.as_str())
            .collect::<Vec<_>>(),
        vec!["01_a", "02_b"]
    );
    assert!(result.orphans.is_empty());
}

#[test]
fn scans_images_and_audio_subdirectories() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("images/01_a.jpg"));
    touch(root.path().join("audio/01_a.mp3"));

    let result = discover(root.path()).unwrap();

    assert_eq!(result.pairs[0].basename, "01_a");
}

#[test]
fn mixes_flat_and_subdir_layouts() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));
    touch(root.path().join("audio/01_a.mp3"));
    touch(root.path().join("images/02_b.png"));
    touch(root.path().join("02_b.wav"));

    let result = discover(root.path()).unwrap();

    assert_eq!(
        result
            .pairs
            .iter()
            .map(|pair| pair.basename.as_str())
            .collect::<Vec<_>>(),
        vec!["01_a", "02_b"]
    );
}

#[test]
fn reports_image_without_audio_as_orphan() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));
    touch(root.path().join("02_b.jpg"));
    touch(root.path().join("02_b.mp3"));

    let result = discover(root.path()).unwrap();

    assert_eq!(result.pairs[0].basename, "02_b");
    assert_eq!(
        result.orphans,
        vec![Orphan {
            kind: OrphanKind::Image,
            basename: "01_a".to_string(),
            path: root.path().join("01_a.jpg"),
        }]
    );
}

#[test]
fn reports_audio_without_image_as_orphan() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.mp3"));
    touch(root.path().join("02_b.jpg"));
    touch(root.path().join("02_b.mp3"));

    let result = discover(root.path()).unwrap();

    assert_eq!(
        result.orphans,
        vec![Orphan {
            kind: OrphanKind::Audio,
            basename: "01_a".to_string(),
            path: root.path().join("01_a.mp3"),
        }]
    );
}

#[test]
fn rejects_duplicate_image_basename() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));
    touch(root.path().join("images/01_a.png"));
    touch(root.path().join("01_a.mp3"));

    let err = discover(root.path()).unwrap_err();

    assert!(err.to_string().contains("ambiguous basename: 01_a"));
}

#[test]
fn rejects_duplicate_audio_basename() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));
    touch(root.path().join("01_a.mp3"));
    touch(root.path().join("audio/01_a.wav"));

    let err = discover(root.path()).unwrap_err();

    assert!(err.to_string().contains("ambiguous basename: 01_a"));
}

#[test]
fn rejects_when_no_pair_exists() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));

    let err = discover(root.path()).unwrap_err();

    assert!(err.to_string().contains("no image/audio pairs found"));
}

#[test]
fn rejects_missing_input_dir() {
    let root = TempDir::new().unwrap();

    let err = discover(root.path().join("missing")).unwrap_err();

    assert!(err.to_string().contains("input dir does not exist"));
}

#[test]
fn ignores_unrelated_extensions() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("01_a.jpg"));
    touch(root.path().join("01_a.mp3"));
    touch(root.path().join("README.txt"));
    touch(root.path().join("01_a.aif"));

    let result = discover(root.path()).unwrap();

    assert_eq!(result.pairs[0].basename, "01_a");
}

#[test]
fn sorts_pairs_and_orphans_by_basename() {
    let root = TempDir::new().unwrap();
    touch(root.path().join("10_x.jpg"));
    touch(root.path().join("10_x.mp3"));
    touch(root.path().join("02_a.jpg"));
    touch(root.path().join("02_a.mp3"));
    touch(root.path().join("03_orphan.jpg"));

    let result = discover(root.path()).unwrap();

    assert_eq!(
        result
            .pairs
            .iter()
            .map(|pair| pair.basename.as_str())
            .collect::<Vec<_>>(),
        vec!["02_a", "10_x"]
    );
    assert_eq!(
        result
            .orphans
            .iter()
            .map(|orphan| orphan.basename.as_str())
            .collect::<Vec<_>>(),
        vec!["03_orphan"]
    );
}
