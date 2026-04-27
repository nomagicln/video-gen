import { describe, it, expect } from 'vitest';
import { parseAudioDurationMs, parseImageSize } from '../../src/ffmpeg/probe.js';

describe('parseAudioDurationMs', () => {
  it('reads duration from streams[0]', () => {
    const json = JSON.stringify({ streams: [{ duration: '4.217333' }] });
    expect(parseAudioDurationMs(json)).toBe(4217);
  });

  it('rounds to nearest ms', () => {
    expect(parseAudioDurationMs(JSON.stringify({ streams: [{ duration: '1.0009' }] }))).toBe(1001);
  });

  it('throws when no streams', () => {
    expect(() => parseAudioDurationMs(JSON.stringify({ streams: [] }))).toThrow(/no audio stream/i);
  });

  it('throws when duration missing', () => {
    expect(() => parseAudioDurationMs(JSON.stringify({ streams: [{}] }))).toThrow(/duration/i);
  });

  it('throws on non-finite duration', () => {
    expect(() => parseAudioDurationMs(JSON.stringify({ streams: [{ duration: 'N/A' }] }))).toThrow(/duration/i);
  });
});

describe('parseImageSize', () => {
  it('reads width/height from first video stream', () => {
    const json = JSON.stringify({ streams: [{ width: 1920, height: 1080 }] });
    expect(parseImageSize(json)).toEqual({ width: 1920, height: 1080 });
  });

  it('throws when stream has no dimensions', () => {
    expect(() => parseImageSize(JSON.stringify({ streams: [{}] }))).toThrow(/dimension/i);
  });
});
