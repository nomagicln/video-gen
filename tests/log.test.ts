import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createLogger } from '../src/log.js';

let stdout: string[] = [];
let stderr: string[] = [];

beforeEach(() => {
  stdout = [];
  stderr = [];
  vi.spyOn(process.stdout, 'write').mockImplementation((chunk: any) => {
    stdout.push(String(chunk));
    return true;
  });
  vi.spyOn(process.stderr, 'write').mockImplementation((chunk: any) => {
    stderr.push(String(chunk));
    return true;
  });
});

describe('logger / text mode', () => {
  it('writes phase lines to stdout', () => {
    const log = createLogger({ mode: 'text' });
    log.discover({ units: 8, orphans: 3 });
    expect(stdout.join('')).toContain('[discover]');
    expect(stdout.join('')).toContain('8 units');
    expect(stdout.join('')).toContain('3 orphans');
  });

  it('writes warn to stderr', () => {
    const log = createLogger({ mode: 'text' });
    log.warn('orphan: 03_a.jpg (no audio)');
    expect(stderr.join('')).toContain('WARN');
    expect(stderr.join('')).toContain('orphan: 03_a.jpg');
  });

  it('writes error to stderr', () => {
    const log = createLogger({ mode: 'text' });
    log.error('image dimensions mismatch');
    expect(stderr.join('')).toContain('[error]');
  });
});

describe('logger / quiet mode', () => {
  it('suppresses progress events', () => {
    const log = createLogger({ mode: 'quiet' });
    log.discover({ units: 8, orphans: 3 });
    log.segment({ index: 1, total: 11, name: 'seg_001', kind: 'lead-in', durationMs: 2000, ms: 400 });
    expect(stdout.join('')).toBe('');
  });

  it('still writes done line', () => {
    const log = createLogger({ mode: 'quiet' });
    log.done({ output: 'output/v.mp4', bytes: 100, durationMs: 1000, ms: 5000 });
    expect(stdout.join('')).toContain('output/v.mp4');
  });

  it('still writes errors to stderr', () => {
    const log = createLogger({ mode: 'quiet' });
    log.error('boom');
    expect(stderr.join('')).toContain('[error]');
  });
});

describe('logger / json mode', () => {
  it('emits one JSON object per event on stdout', () => {
    const log = createLogger({ mode: 'json' });
    log.discover({ units: 8, orphans: 3 });
    const lines = stdout.join('').trim().split('\n');
    expect(JSON.parse(lines[0]!)).toEqual({ phase: 'discover', units: 8, orphans: 3 });
  });

  it('emits errors as json lines on stderr', () => {
    const log = createLogger({ mode: 'json' });
    log.error('boom', 2);
    expect(JSON.parse(stderr.join('').trim())).toEqual({ phase: 'error', code: 2, message: 'boom' });
  });
});
