import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { resolveBinary } from '../../src/ffmpeg/resolve.js';

const ENV_KEYS = ['VIDEO_GEN_FFMPEG', 'VIDEO_GEN_FFPROBE'] as const;
const savedEnv: Record<string, string | undefined> = {};

beforeEach(() => {
  for (const k of ENV_KEYS) savedEnv[k] = process.env[k];
});
afterEach(() => {
  for (const k of ENV_KEYS) {
    if (savedEnv[k] === undefined) delete process.env[k];
    else process.env[k] = savedEnv[k];
  }
});

describe('resolveBinary', () => {
  it('returns env override when set and non-empty', () => {
    process.env.VIDEO_GEN_FFMPEG = '/custom/ffmpeg';
    expect(resolveBinary('ffmpeg')).toBe('/custom/ffmpeg');
  });

  it('ignores empty env override', () => {
    process.env.VIDEO_GEN_FFMPEG = '   ';
    expect(resolveBinary('ffmpeg', { execDir: '/no/where' }))
      .toBe(process.platform === 'win32' ? 'ffmpeg.exe' : 'ffmpeg');
  });

  it('falls back to sibling of execPath when present', () => {
    delete process.env.VIDEO_GEN_FFMPEG;
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vg-'));
    const exe = process.platform === 'win32' ? 'ffmpeg.exe' : 'ffmpeg';
    const sibling = path.join(tmp, exe);
    fs.writeFileSync(sibling, '');
    try {
      expect(resolveBinary('ffmpeg', { execDir: tmp })).toBe(sibling);
    } finally {
      fs.rmSync(tmp, { recursive: true });
    }
  });

  it('falls back to bare name (relying on $PATH)', () => {
    delete process.env.VIDEO_GEN_FFMPEG;
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vg-'));
    try {
      expect(resolveBinary('ffmpeg', { execDir: tmp }))
        .toBe(process.platform === 'win32' ? 'ffmpeg.exe' : 'ffmpeg');
    } finally {
      fs.rmSync(tmp, { recursive: true });
    }
  });

  it('reads VIDEO_GEN_FFPROBE for ffprobe', () => {
    process.env.VIDEO_GEN_FFPROBE = '/custom/ffprobe';
    expect(resolveBinary('ffprobe')).toBe('/custom/ffprobe');
  });
});
