import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { makeRunId, BuildDir } from '../../src/store/paths.js';

let cwd: string;

beforeEach(() => {
  cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'vg-store-'));
});
afterEach(() => {
  fs.rmSync(cwd, { recursive: true });
});

describe('makeRunId', () => {
  it('formats build-YYYYMMDD-HHMMSS', () => {
    const id = makeRunId(new Date('2026-04-27T10:42:01Z'));
    expect(id).toMatch(/^build-\d{8}-\d{6}$/);
  });
});

describe('BuildDir', () => {
  it('creates .video-gen/<runId> on init', () => {
    const bd = new BuildDir(cwd, 'build-test');
    bd.init();
    expect(fs.existsSync(path.join(cwd, '.video-gen', 'build-test'))).toBe(true);
  });

  it('segPath returns predictable name', () => {
    const bd = new BuildDir(cwd, 'build-test');
    expect(bd.segPath(2)).toBe(path.join(cwd, '.video-gen', 'build-test', 'seg_002.mp4'));
    expect(bd.segPath(123)).toBe(path.join(cwd, '.video-gen', 'build-test', 'seg_123.mp4'));
  });

  it('concatListPath is concat.txt inside build dir', () => {
    const bd = new BuildDir(cwd, 'build-test');
    expect(bd.concatListPath()).toBe(path.join(cwd, '.video-gen', 'build-test', 'concat.txt'));
  });

  it('cleanup removes the build dir', () => {
    const bd = new BuildDir(cwd, 'build-test');
    bd.init();
    bd.cleanup();
    expect(fs.existsSync(path.join(cwd, '.video-gen', 'build-test'))).toBe(false);
  });

  it('cleanup is idempotent', () => {
    const bd = new BuildDir(cwd, 'build-test');
    expect(() => bd.cleanup()).not.toThrow();
  });
});
