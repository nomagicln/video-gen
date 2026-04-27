import type { Segment } from '../pipeline/plan.js';

export type EncodeOptions = {
  fps: number;
  crf: number;
  preset: string;
  audioBitrate: string;
};

const formatSeconds = (ms: number) => (ms / 1000).toFixed(3);

export function segmentArgv(seg: Segment, outputPath: string, opts: EncodeOptions): string[] {
  const argv: string[] = ['-y', '-loop', '1', '-framerate', String(opts.fps), '-i', seg.image];

  if (seg.kind === 'unit') {
    argv.push('-i', seg.audio, '-af', 'aresample=48000,aformat=channel_layouts=stereo');
  } else {
    argv.push('-f', 'lavfi', '-i', 'aevalsrc=0:s=48000:c=stereo');
  }

  argv.push(
    '-c:v', 'libx264',
    '-preset', opts.preset,
    '-crf', String(opts.crf),
    '-pix_fmt', 'yuv420p',
    '-c:a', 'aac',
    '-b:a', opts.audioBitrate,
    '-t', formatSeconds(seg.durationMs),
    '-shortest',
    '-movflags', '+faststart',
    outputPath,
  );
  return argv;
}

export function concatArgv(concatListPath: string, outputPath: string): string[] {
  return [
    '-y',
    '-f', 'concat', '-safe', '0',
    '-i', concatListPath,
    '-c', 'copy',
    '-movflags', '+faststart',
    outputPath,
  ];
}

export function concatListContent(segmentFilenames: string[]): string {
  return segmentFilenames.map((n) => `file '${n.replace(/'/g, `'\\''`)}'`).join('\n') + '\n';
}
