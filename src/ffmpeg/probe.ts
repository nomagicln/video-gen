import { spawn } from 'node:child_process';
import { resolveBinary, type Tool } from './resolve.js';
import { UserError } from '../errors.js';

export function parseAudioDurationMs(jsonText: string): number {
  const data = JSON.parse(jsonText) as { streams?: Array<{ duration?: string }> };
  const s = data.streams?.[0];
  if (!s) throw new Error('ffprobe: no audio stream found');
  if (!s.duration) throw new Error('ffprobe: missing audio duration');
  const sec = Number(s.duration);
  if (!Number.isFinite(sec)) throw new Error(`ffprobe: invalid audio duration "${s.duration}"`);
  return Math.round(sec * 1000);
}

export function parseImageSize(jsonText: string): { width: number; height: number } {
  const data = JSON.parse(jsonText) as { streams?: Array<{ width?: number; height?: number }> };
  const s = data.streams?.[0];
  if (!s || typeof s.width !== 'number' || typeof s.height !== 'number') {
    throw new Error('ffprobe: missing image dimensions');
  }
  return { width: s.width, height: s.height };
}

async function runFfprobe(args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const bin = resolveBinary('ffprobe');
    const proc = spawn(bin, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    proc.stdout.on('data', (d) => stdout.push(Buffer.from(d)));
    proc.stderr.on('data', (d) => stderr.push(Buffer.from(d)));
    proc.on('error', reject);
    proc.on('close', (code) => {
      if (code === 0) resolve(Buffer.concat(stdout).toString('utf8'));
      else reject(new Error(`ffprobe exit ${code}: ${Buffer.concat(stderr).toString('utf8').slice(-500)}`));
    });
  });
}

export async function probeAudioDurationMs(file: string): Promise<number> {
  const json = await runFfprobe([
    '-v', 'error',
    '-of', 'json',
    '-select_streams', 'a:0',
    '-show_entries', 'stream=duration',
    file,
  ]);
  return parseAudioDurationMs(json);
}

export async function probeImageSize(file: string): Promise<{ width: number; height: number }> {
  const json = await runFfprobe([
    '-v', 'error',
    '-of', 'json',
    '-select_streams', 'v:0',
    '-show_entries', 'stream=width,height',
    file,
  ]);
  return parseImageSize(json);
}

export async function checkBinary(tool: Tool): Promise<void> {
  return new Promise((resolve, reject) => {
    const bin = resolveBinary(tool);
    const proc = spawn(bin, ['-version'], { stdio: ['ignore', 'ignore', 'pipe'] });
    let stderrBuf = '';
    proc.stderr.on('data', (d) => { stderrBuf += d.toString(); });
    proc.on('error', () => reject(new UserError(`${tool} not found at "${bin}". Install ffmpeg or set VIDEO_GEN_${tool.toUpperCase()}.`)));
    proc.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new UserError(`${tool} -version failed (exit ${code}): ${stderrBuf.slice(-200)}`));
    });
  });
}
