import { describe, it, expect } from 'vitest';
import { segmentArgv, concatArgv, concatListContent, type EncodeOptions } from '../../src/ffmpeg/encode.js';

const opts: EncodeOptions = { fps: 30, crf: 20, preset: 'medium', audioBitrate: '192k' };

describe('segmentArgv', () => {
  it('builds unit segment with audio input', () => {
    const argv = segmentArgv(
      { kind: 'unit', image: '/i/a.jpg', audio: '/a/a.mp3', durationMs: 4217 },
      '/tmp/seg_002.mp4',
      opts,
    );
    expect(argv).toEqual([
      '-y',
      '-loop', '1', '-framerate', '30', '-i', '/i/a.jpg',
      '-i', '/a/a.mp3',
      '-af', 'aresample=48000,aformat=channel_layouts=stereo',
      '-c:v', 'libx264', '-preset', 'medium', '-crf', '20', '-pix_fmt', 'yuv420p',
      '-c:a', 'aac', '-b:a', '192k',
      '-t', '4.217', '-shortest',
      '-movflags', '+faststart',
      '/tmp/seg_002.mp4',
    ]);
  });

  it('builds silent (lead-in/gap/tail) segment with aevalsrc', () => {
    const argv = segmentArgv(
      { kind: 'lead-in', image: '/i/a.jpg', durationMs: 2000 },
      '/tmp/seg_001.mp4',
      opts,
    );
    expect(argv).toEqual([
      '-y',
      '-loop', '1', '-framerate', '30', '-i', '/i/a.jpg',
      '-f', 'lavfi', '-i', 'aevalsrc=0:s=48000:c=stereo',
      '-c:v', 'libx264', '-preset', 'medium', '-crf', '20', '-pix_fmt', 'yuv420p',
      '-c:a', 'aac', '-b:a', '192k',
      '-t', '2.000', '-shortest',
      '-movflags', '+faststart',
      '/tmp/seg_001.mp4',
    ]);
  });

  it('formats fractional seconds to 3 decimals', () => {
    const argv = segmentArgv(
      { kind: 'gap', image: '/i/x.jpg', durationMs: 500 },
      '/tmp/seg.mp4',
      opts,
    );
    expect(argv).toContain('0.500');
  });

  it('respects custom encode options', () => {
    const argv = segmentArgv(
      { kind: 'gap', image: '/i/x.jpg', durationMs: 500 },
      '/tmp/seg.mp4',
      { fps: 25, crf: 18, preset: 'slower', audioBitrate: '256k' },
    );
    expect(argv).toContain('25');
    expect(argv).toContain('18');
    expect(argv).toContain('slower');
    expect(argv).toContain('256k');
  });
});

describe('concatArgv', () => {
  it('uses concat demuxer with copy codec', () => {
    expect(concatArgv('/tmp/concat.txt', '/out/v.mp4')).toEqual([
      '-y',
      '-f', 'concat', '-safe', '0',
      '-i', '/tmp/concat.txt',
      '-c', 'copy',
      '-movflags', '+faststart',
      '/out/v.mp4',
    ]);
  });
});

describe('concatListContent', () => {
  it('emits one file line per segment, trailing newline', () => {
    expect(concatListContent(['seg_001.mp4', 'seg_002.mp4'])).toBe(
      "file 'seg_001.mp4'\nfile 'seg_002.mp4'\n",
    );
  });
});
