import fs from 'node:fs';
import path from 'node:path';

const IMAGE_EXT = new Set(['.jpg', '.jpeg', '.png', '.webp']);
const AUDIO_EXT = new Set(['.mp3', '.wav', '.m4a', '.flac']);

export type Pair = { basename: string; image: string; audio: string };
export type Orphan = { kind: 'image' | 'audio'; basename: string; path: string };

export type DiscoverResult = { pairs: Pair[]; orphans: Orphan[] };

const stripExt = (filename: string) => path.basename(filename, path.extname(filename));

function listDirSafe(dir: string): string[] {
  try {
    if (!fs.statSync(dir).isDirectory()) return [];
  } catch {
    return [];
  }
  return fs.readdirSync(dir).map((n) => path.join(dir, n));
}

export function discover(rootDir: string): DiscoverResult {
  if (!fs.existsSync(rootDir)) {
    throw new Error(`input dir does not exist: ${rootDir}`);
  }

  const candidates = [
    ...listDirSafe(rootDir),
    ...listDirSafe(path.join(rootDir, 'images')),
    ...listDirSafe(path.join(rootDir, 'audio')),
  ];

  const images = new Map<string, string[]>();
  const audios = new Map<string, string[]>();

  for (const file of candidates) {
    let stat: fs.Stats;
    try {
      stat = fs.statSync(file);
    } catch {
      continue;
    }
    if (!stat.isFile()) continue;
    const ext = path.extname(file).toLowerCase();
    const base = stripExt(file);
    if (IMAGE_EXT.has(ext)) {
      const arr = images.get(base) ?? [];
      arr.push(file);
      images.set(base, arr);
    } else if (AUDIO_EXT.has(ext)) {
      const arr = audios.get(base) ?? [];
      arr.push(file);
      audios.set(base, arr);
    }
  }

  for (const [base, arr] of images) {
    if (arr.length > 1) {
      throw new Error(
        `ambiguous basename: ${base}\n  matched ${arr.length} images: ${arr.join(', ')}\n  v0.1 only supports 1:1 pairing — 请合并或重命名`,
      );
    }
  }
  for (const [base, arr] of audios) {
    if (arr.length > 1) {
      throw new Error(
        `ambiguous basename: ${base}\n  matched ${arr.length} audios: ${arr.join(', ')}\n  v0.1 only supports 1:1 pairing — 请合并或重命名`,
      );
    }
  }

  const pairs: Pair[] = [];
  const orphans: Orphan[] = [];

  const allBases = new Set<string>([...images.keys(), ...audios.keys()]);
  for (const base of allBases) {
    const img = images.get(base)?.[0];
    const aud = audios.get(base)?.[0];
    if (img && aud) {
      pairs.push({ basename: base, image: img, audio: aud });
    } else if (img) {
      orphans.push({ kind: 'image', basename: base, path: img });
    } else if (aud) {
      orphans.push({ kind: 'audio', basename: base, path: aud });
    }
  }

  pairs.sort((a, b) => a.basename.localeCompare(b.basename));
  orphans.sort((a, b) => a.basename.localeCompare(b.basename));

  if (pairs.length === 0) {
    throw new Error(`no image/audio pairs found in ${rootDir}`);
  }

  return { pairs, orphans };
}
