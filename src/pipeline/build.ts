import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { discover } from './discover.js';
import { plan, type PlanOptions, type Segment, type Unit } from './plan.js';
import { resolveBinary } from '../ffmpeg/resolve.js';
import { checkBinary, probeAudioDurationMs, probeImageSize } from '../ffmpeg/probe.js';
import { segmentArgv, concatArgv, concatListContent, type EncodeOptions } from '../ffmpeg/encode.js';
import { BuildDir, makeRunId } from '../store/paths.js';
import type { Logger } from '../log.js';

export type BuildOptions = {
  inputDir: string;
  outputPath: string;
  cwd: string;
  keepTemp: boolean;
  plan: PlanOptions;
  encode: EncodeOptions;
  logger: Logger;
};

type SpawnResult = { code: number; stderr: string };

function runFfmpeg(argv: string[]): Promise<SpawnResult> {
  return new Promise((resolve, reject) => {
    const bin = resolveBinary('ffmpeg');
    const proc = spawn(bin, argv, { stdio: ['ignore', 'ignore', 'pipe'] });
    let stderr = '';
    proc.stderr.on('data', (d) => { stderr += d.toString(); });
    proc.on('error', reject);
    proc.on('close', (code) => resolve({ code: code ?? -1, stderr }));
  });
}

const lastLines = (s: string, n: number) => s.trim().split('\n').slice(-n).join('\n');

function summarize(segments: Segment[]): string {
  const counts: Record<string, number> = { 'lead-in': 0, unit: 0, gap: 0, tail: 0 };
  let gapMs = 0;
  for (const s of segments) {
    counts[s.kind]!++;
    if (s.kind === 'gap') gapMs = s.durationMs;
  }
  const parts: string[] = [];
  const lead = segments.find((s) => s.kind === 'lead-in');
  if (lead) parts.push(`lead-in ${(lead.durationMs / 1000).toFixed(1)}s`);
  parts.push(`${counts.unit}x unit`);
  if (counts.gap > 0) parts.push(`${counts.gap}x gap ${(gapMs / 1000).toFixed(1)}s`);
  const tail = segments.find((s) => s.kind === 'tail');
  if (tail) parts.push(`tail ${(tail.durationMs / 1000).toFixed(1)}s`);
  return parts.join(', ');
}

export async function build(opts: BuildOptions): Promise<{ output: string }> {
  const { logger } = opts;
  const t0 = Date.now();

  await checkBinary('ffmpeg');
  await checkBinary('ffprobe');

  const { pairs, orphans } = discover(opts.inputDir);
  for (const o of orphans) {
    logger.warn(`orphan: ${path.basename(o.path)} (no ${o.kind === 'image' ? 'audio' : 'image'})`);
  }
  logger.discover({ units: pairs.length, orphans: orphans.length });

  const sizes = await Promise.all(pairs.map((p) => probeImageSize(p.image)));
  const ref = sizes[0]!;
  for (let i = 1; i < sizes.length; i++) {
    const s = sizes[i]!;
    if (s.width !== ref.width || s.height !== ref.height) {
      throw new Error(
        `image dimensions mismatch:\n  expected ${ref.width}x${ref.height} (from ${path.basename(pairs[0]!.image)})\n  got      ${s.width}x${s.height}  in    ${path.basename(pairs[i]!.image)}`,
      );
    }
  }

  const units: Unit[] = await Promise.all(pairs.map(async (p) => ({
    basename: p.basename,
    imagePath: p.image,
    audioPath: p.audio,
    audioDurationMs: await probeAudioDurationMs(p.audio),
  })));

  const segments = plan(units, opts.plan);
  const totalMs = segments.reduce((sum, s) => sum + s.durationMs, 0);
  logger.plan({ segments: segments.length, totalMs, summary: summarize(segments) });

  const runId = makeRunId();
  const buildDir = new BuildDir(opts.cwd, runId);
  buildDir.init();

  let success = false;
  try {
    const segmentNames: string[] = [];
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i]!;
      const segIdx = i + 1;
      const segPath = buildDir.segPath(segIdx);
      const segName = path.basename(segPath, '.mp4');
      const segStart = Date.now();
      const argv = segmentArgv(seg, segPath, opts.encode);
      const r = await runFfmpeg(argv);
      const segMs = Date.now() - segStart;
      const basenameFor = seg.kind === 'unit'
        ? units.find((u) => u.imagePath === seg.image)?.basename
        : undefined;
      if (r.code !== 0) {
        logger.segment({
          index: segIdx, total: segments.length, name: segName, kind: seg.kind,
          basename: basenameFor,
          durationMs: seg.durationMs, ms: segMs, status: 'fail',
        });
        throw new Error(`segment ${segName} (${seg.kind}) failed (ffmpeg exit ${r.code}):\n${lastLines(r.stderr, 20)}`);
      }
      logger.segment({
        index: segIdx, total: segments.length, name: segName, kind: seg.kind,
        basename: basenameFor,
        durationMs: seg.durationMs, ms: segMs, status: 'ok',
      });
      segmentNames.push(path.basename(segPath));
    }

    const listPath = buildDir.concatListPath();
    fs.writeFileSync(listPath, concatListContent(segmentNames));
    fs.mkdirSync(path.dirname(opts.outputPath), { recursive: true });
    const r = await runFfmpeg(concatArgv(listPath, opts.outputPath));
    if (r.code !== 0) {
      throw new Error(`concat failed (ffmpeg exit ${r.code}):\n${lastLines(r.stderr, 20)}`);
    }

    const bytes = fs.statSync(opts.outputPath).size;
    logger.concat({ output: opts.outputPath, bytes, durationMs: totalMs });
    logger.done({ output: opts.outputPath, bytes, durationMs: totalMs, ms: Date.now() - t0 });

    success = true;
    return { output: opts.outputPath };
  } finally {
    if (success && !opts.keepTemp) {
      buildDir.cleanup();
    }
  }
}
