import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { build } from '../../src/pipeline/build.js';
import { createLogger } from '../../src/log.js';
import { hasFfmpeg, makeImage, makeSilentWav, probeDurationSec } from './fixtures.js';

const itIfFfmpeg = hasFfmpeg ? it : it.skip;

let workDir: string;

beforeEach(() => {
  workDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vg-e2e-'));
});
afterEach(() => {
  fs.rmSync(workDir, { recursive: true });
});

describe('e2e build', () => {
  itIfFfmpeg('produces a single mp4 of expected duration with lead-in/tail/gap', async () => {
    const inputDir = path.join(workDir, 'input');
    const outputPath = path.join(workDir, 'output', 'test.mp4');

    await makeImage(path.join(inputDir, '01_a.png'), 320, 180, 'red');
    await makeImage(path.join(inputDir, '02_b.png'), 320, 180, 'blue');
    await makeSilentWav(path.join(inputDir, '01_a.wav'), 1.0);
    await makeSilentWav(path.join(inputDir, '02_b.wav'), 1.0);

    await build({
      inputDir,
      outputPath,
      cwd: workDir,
      keepTemp: false,
      plan: { leadInMs: 1000, tailMs: 1000, gapMs: 500 },
      encode: { fps: 30, crf: 28, preset: 'ultrafast', audioBitrate: '128k' },
      logger: createLogger({ mode: 'quiet' }),
    });

    expect(fs.existsSync(outputPath)).toBe(true);
    const dur = await probeDurationSec(outputPath);
    expect(dur).toBeGreaterThan(4.3);
    expect(dur).toBeLessThan(4.8);

    expect(fs.existsSync(path.join(workDir, '.video-gen'))).toBe(true);
    const buildDirs = fs.readdirSync(path.join(workDir, '.video-gen'));
    expect(buildDirs).toHaveLength(0);
  }, 60_000);

  itIfFfmpeg('rejects mismatched image dimensions', async () => {
    const inputDir = path.join(workDir, 'input');
    const outputPath = path.join(workDir, 'output', 'test.mp4');

    await makeImage(path.join(inputDir, '01_a.png'), 320, 180);
    await makeImage(path.join(inputDir, '02_b.png'), 640, 360);
    await makeSilentWav(path.join(inputDir, '01_a.wav'), 1.0);
    await makeSilentWav(path.join(inputDir, '02_b.wav'), 1.0);

    await expect(build({
      inputDir,
      outputPath,
      cwd: workDir,
      keepTemp: false,
      plan: { leadInMs: 0, tailMs: 0, gapMs: 0 },
      encode: { fps: 30, crf: 28, preset: 'ultrafast', audioBitrate: '128k' },
      logger: createLogger({ mode: 'quiet' }),
    })).rejects.toThrow(/dimensions mismatch/);
  }, 30_000);
});
