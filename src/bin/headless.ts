import { Command } from 'commander';
import fs from 'node:fs';
import path from 'node:path';
import { build } from '../pipeline/build.js';
import { createLogger, type LogMode } from '../log.js';
import { UserError } from '../errors.js';

type BuildOpts = {
  inputDir: string;
  output?: string;
  leadIn: string;
  tail: string;
  gap: string;
  fps: string;
  crf: string;
  preset: string;
  audioBitrate: string;
  keepTemp?: boolean;
  quiet?: boolean;
  json?: boolean;
};

function parseSeconds(label: string, value: string): number {
  const n = Number(value);
  if (!Number.isFinite(n) || n < 0) throw new UserError(`--${label}: expected a non-negative number, got "${value}"`);
  return Math.round(n * 1000);
}

function parseInteger(label: string, value: string, min: number, max: number): number {
  const n = Number(value);
  if (!Number.isInteger(n) || n < min || n > max) {
    throw new UserError(`--${label}: expected integer in [${min},${max}], got "${value}"`);
  }
  return n;
}

export function resolveOutputPath(rawOutput: string | undefined, inputDir: string): string {
  const fallback = path.join('output', `${path.basename(inputDir)}.mp4`);
  const resolved = path.resolve(rawOutput ?? fallback);
  let isDir = false;
  try {
    isDir = fs.statSync(resolved).isDirectory();
  } catch {
    isDir =
      rawOutput !== undefined &&
      (/[\\/]$/.test(rawOutput) || path.extname(resolved) === '');
  }
  return isDir ? path.join(resolved, 'output.mp4') : resolved;
}

export function buildHeadlessProgram(): Command {
  const program = new Command('video-gen').description('image+audio → mp4 video composer');

  program
    .command('build')
    .description('compose images and audio into a single mp4')
    .option('-d, --input-dir <dir>', 'input directory (also scans <dir>/images/ and <dir>/audio/)', 'input')
    .option('-o, --output <path>', 'output mp4 path (default: output/<input-dir-basename>.mp4)')
    .option('--lead-in <sec>', 'leading silence (shows first image)', '0')
    .option('--tail <sec>', 'trailing silence (shows last image)', '0')
    .option('--gap <sec>', 'silence between units (shows preceding image)', '0')
    .option('--fps <n>', 'video framerate', '30')
    .option('--crf <n>', 'x264 crf (lower=better, 0-51)', '20')
    .option('--preset <name>', 'x264 preset', 'medium')
    .option('--audio-bitrate <bps>', 'aac bitrate', '192k')
    .option('--keep-temp', 'preserve .video-gen/<runId>/ on success')
    .option('--quiet', 'suppress progress; print only final result')
    .option('--json', 'emit one JSON object per phase event')
    .action(async (opts: BuildOpts) => {
      const mode: LogMode = opts.json ? 'json' : opts.quiet ? 'quiet' : 'text';
      const logger = createLogger({ mode });

      try {
        const inputDir = path.resolve(opts.inputDir);
        const outputPath = resolveOutputPath(opts.output, inputDir);
        await build({
          inputDir,
          outputPath,
          cwd: process.cwd(),
          keepTemp: !!opts.keepTemp,
          plan: {
            leadInMs: parseSeconds('lead-in', opts.leadIn),
            tailMs: parseSeconds('tail', opts.tail),
            gapMs: parseSeconds('gap', opts.gap),
          },
          encode: {
            fps: parseInteger('fps', opts.fps, 1, 240),
            crf: parseInteger('crf', opts.crf, 0, 51),
            preset: opts.preset,
            audioBitrate: opts.audioBitrate,
          },
          logger,
        });
      } catch (err) {
        const code = err instanceof UserError ? err.code : 1;
        const msg = err instanceof Error ? err.message : String(err);
        logger.error(msg, code);
        process.exit(code);
      }
    });

  return program;
}
