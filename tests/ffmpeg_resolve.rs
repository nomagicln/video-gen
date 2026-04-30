use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use tempfile::TempDir;
use video_gen::ffmpeg::{resolve_binary, Tool};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

#[test]
fn explicit_binary_path_wins() {
    let _guard = env_lock().lock().unwrap();
    let path = Path::new("/custom/ffmpeg");

    assert_eq!(
        resolve_binary(Tool::Ffmpeg, Some(path), Some(Path::new("/no/where"))).unwrap(),
        path
    );
}

#[test]
fn env_override_is_used_when_set_and_non_empty() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("VIDEO_GEN_FFMPEG", "/env/ffmpeg");

    let result = resolve_binary(Tool::Ffmpeg, None, Some(Path::new("/no/where"))).unwrap();

    std::env::remove_var("VIDEO_GEN_FFMPEG");
    assert_eq!(result, Path::new("/env/ffmpeg"));
}

#[test]
fn empty_env_override_is_ignored() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("VIDEO_GEN_FFMPEG", "   ");
    let dir = TempDir::new().unwrap();

    let result = resolve_binary(Tool::Ffmpeg, None, Some(dir.path())).unwrap();

    std::env::remove_var("VIDEO_GEN_FFMPEG");
    assert_eq!(result, Path::new(&exe_name("ffmpeg")));
}

#[test]
fn sibling_binary_wins_before_path_lookup() {
    let _guard = env_lock().lock().unwrap();
    std::env::remove_var("VIDEO_GEN_FFMPEG");
    let dir = TempDir::new().unwrap();
    let sibling = dir.path().join(exe_name("ffmpeg"));
    fs::write(&sibling, "").unwrap();

    let result = resolve_binary(Tool::Ffmpeg, None, Some(dir.path())).unwrap();

    assert_eq!(result, sibling);
}

#[test]
fn falls_back_to_bare_name_for_path_lookup() {
    let _guard = env_lock().lock().unwrap();
    std::env::remove_var("VIDEO_GEN_FFMPEG");
    let dir = TempDir::new().unwrap();

    let result = resolve_binary(Tool::Ffmpeg, None, Some(dir.path())).unwrap();

    assert_eq!(result, Path::new(&exe_name("ffmpeg")));
}

#[test]
fn ffprobe_uses_ffprobe_env_key() {
    let _guard = env_lock().lock().unwrap();
    std::env::set_var("VIDEO_GEN_FFPROBE", "/env/ffprobe");

    let result = resolve_binary(Tool::Ffprobe, None, Some(Path::new("/no/where"))).unwrap();

    std::env::remove_var("VIDEO_GEN_FFPROBE");
    assert_eq!(result, Path::new("/env/ffprobe"));
}
