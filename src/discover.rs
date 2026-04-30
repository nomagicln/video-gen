use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::VideoGenError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    pub basename: String,
    pub image: PathBuf,
    pub audio: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrphanKind {
    Image,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Orphan {
    pub kind: OrphanKind,
    pub basename: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverResult {
    pub pairs: Vec<Pair>,
    pub orphans: Vec<Orphan>,
}

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const AUDIO_EXTS: &[&str] = &["mp3", "wav", "m4a", "flac"];

fn list_dir_safe(dir: &Path) -> Vec<PathBuf> {
    let Ok(metadata) = fs::metadata(dir) else {
        return Vec::new();
    };
    if !metadata.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect()
}

fn push_match(map: &mut HashMap<String, Vec<PathBuf>>, base: String, path: PathBuf) {
    map.entry(base).or_default().push(path);
}

fn basename(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn reject_duplicates(
    label: &str,
    entries: &HashMap<String, Vec<PathBuf>>,
) -> Result<(), VideoGenError> {
    for (base, paths) in entries {
        if paths.len() > 1 {
            let joined = paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(VideoGenError::user(format!(
                "ambiguous basename: {base}\n  matched {} {label}: {joined}\n  v0.1 only supports 1:1 pairing - 请合并或重命名",
                paths.len()
            )));
        }
    }
    Ok(())
}

pub fn discover(root_dir: impl AsRef<Path>) -> Result<DiscoverResult, VideoGenError> {
    let root_dir = root_dir.as_ref();
    if !root_dir.exists() {
        return Err(VideoGenError::user(format!(
            "input dir does not exist: {}",
            root_dir.display()
        )));
    }

    let mut candidates = Vec::new();
    candidates.extend(list_dir_safe(root_dir));
    candidates.extend(list_dir_safe(&root_dir.join("images")));
    candidates.extend(list_dir_safe(&root_dir.join("audio")));

    let mut images: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut audios: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for file in candidates {
        let Ok(metadata) = fs::metadata(&file) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Some(ext) = extension(&file) else {
            continue;
        };
        let Some(base) = basename(&file) else {
            continue;
        };

        if IMAGE_EXTS.contains(&ext.as_str()) {
            push_match(&mut images, base, file);
        } else if AUDIO_EXTS.contains(&ext.as_str()) {
            push_match(&mut audios, base, file);
        }
    }

    reject_duplicates("images", &images)?;
    reject_duplicates("audios", &audios)?;

    let bases = images
        .keys()
        .chain(audios.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut pairs = Vec::new();
    let mut orphans = Vec::new();
    for base in bases {
        let image = images.get(&base).and_then(|paths| paths.first()).cloned();
        let audio = audios.get(&base).and_then(|paths| paths.first()).cloned();
        match (image, audio) {
            (Some(image), Some(audio)) => pairs.push(Pair {
                basename: base,
                image,
                audio,
            }),
            (Some(path), None) => orphans.push(Orphan {
                kind: OrphanKind::Image,
                basename: base,
                path,
            }),
            (None, Some(path)) => orphans.push(Orphan {
                kind: OrphanKind::Audio,
                basename: base,
                path,
            }),
            (None, None) => {}
        }
    }

    if pairs.is_empty() {
        return Err(VideoGenError::user(format!(
            "no image/audio pairs found in {}",
            root_dir.display()
        )));
    }

    Ok(DiscoverResult { pairs, orphans })
}
