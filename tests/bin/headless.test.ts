import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { resolveOutputPath } from '../../src/bin/headless.js';

let workDir: string;
const inputDir = '/tmp/some-input';

beforeEach(() => {
  workDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vg-headless-'));
});
afterEach(() => {
  fs.rmSync(workDir, { recursive: true, force: true });
});

describe('resolveOutputPath', () => {
  it('uses default output/<input-basename>.mp4 when --output is omitted', () => {
    const out = resolveOutputPath(undefined, inputDir);
    expect(out).toBe(path.resolve('output', 'some-input.mp4'));
  });

  it('returns the given path unchanged when it points at a file', () => {
    const out = resolveOutputPath(path.join(workDir, 'movie.mp4'), inputDir);
    expect(out).toBe(path.join(workDir, 'movie.mp4'));
  });

  it('appends output.mp4 when the given path is an existing directory', () => {
    const out = resolveOutputPath(workDir, inputDir);
    expect(out).toBe(path.join(workDir, 'output.mp4'));
  });

  it('appends output.mp4 when the path ends with a trailing slash even if absent', () => {
    const ghost = path.join(workDir, 'not-yet-created') + path.sep;
    const out = resolveOutputPath(ghost, inputDir);
    expect(out).toBe(path.join(workDir, 'not-yet-created', 'output.mp4'));
  });

  it('treats an absent extensionless path as a directory and appends output.mp4', () => {
    const ghost = path.join(workDir, 'not-yet-created');
    const out = resolveOutputPath(ghost, inputDir);
    expect(out).toBe(path.join(workDir, 'not-yet-created', 'output.mp4'));
  });

  it('keeps an absent path with an extension as a file path', () => {
    const ghost = path.join(workDir, 'unborn', 'movie.mp4');
    const out = resolveOutputPath(ghost, inputDir);
    expect(out).toBe(ghost);
  });
});
