import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { resolveBinary } from '../../src/ffmpeg/resolve.js';

const FFMPEG = resolveBinary('ffmpeg');
const FFPROBE = resolveBinary('ffprobe');

export const hasFfmpeg = (() => {
  try {
    const r = spawnSync(FFMPEG, ['-version'], { stdio: 'ignore' });
    return r.status === 0;
  } catch {
    return false;
  }
})();

function run(bin: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const p = spawn(bin, args, { stdio: ['ignore', 'ignore', 'pipe'] });
    let err = '';
    p.stderr.on('data', (d) => { err += d.toString(); });
    p.on('close', (code) => code === 0 ? resolve() : reject(new Error(`${bin} exit ${code}: ${err}`)));
  });
}

export async function makeImage(file: string, width = 1920, height = 1080, color = 'red'): Promise<void> {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  await run(FFMPEG, ['-y', '-f', 'lavfi', '-i', `color=c=${color}:s=${width}x${height}:d=1`, '-frames:v', '1', file]);
}

export async function makeSilentWav(file: string, durationSec: number): Promise<void> {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  await run(FFMPEG, [
    '-y',
    '-f', 'lavfi', '-i', `anullsrc=r=44100:cl=stereo`,
    '-t', String(durationSec),
    file,
  ]);
}

export async function probeDurationSec(file: string): Promise<number> {
  return new Promise((resolve, reject) => {
    const p = spawn(FFPROBE, [
      '-v', 'error', '-of', 'json',
      '-show_entries', 'format=duration',
      file,
    ], { stdio: ['ignore', 'pipe', 'pipe'] });
    const chunks: Buffer[] = [];
    let err = '';
    p.stdout.on('data', (d) => chunks.push(Buffer.from(d)));
    p.stderr.on('data', (d) => { err += d.toString(); });
    p.on('close', (code) => {
      if (code !== 0) return reject(new Error(`ffprobe exit ${code}: ${err}`));
      try {
        const obj = JSON.parse(Buffer.concat(chunks).toString('utf8')) as { format?: { duration?: string } };
        resolve(Number(obj.format?.duration));
      } catch (e) {
        reject(e);
      }
    });
  });
}
