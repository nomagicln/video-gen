import fs from 'node:fs';
import path from 'node:path';

export function makeRunId(now: Date = new Date()): string {
  const pad = (n: number, w = 2) => String(n).padStart(w, '0');
  const yyyy = now.getFullYear();
  const mm = pad(now.getMonth() + 1);
  const dd = pad(now.getDate());
  const hh = pad(now.getHours());
  const mi = pad(now.getMinutes());
  const ss = pad(now.getSeconds());
  return `build-${yyyy}${mm}${dd}-${hh}${mi}${ss}`;
}

export class BuildDir {
  private readonly base: string;

  constructor(rootCwd: string, public readonly runId: string) {
    this.base = path.join(rootCwd, '.video-gen', runId);
  }

  init(): void {
    fs.mkdirSync(this.base, { recursive: true });
  }

  get path(): string {
    return this.base;
  }

  segPath(index: number): string {
    return path.join(this.base, `seg_${String(index).padStart(3, '0')}.mp4`);
  }

  concatListPath(): string {
    return path.join(this.base, 'concat.txt');
  }

  cleanup(): void {
    fs.rmSync(this.base, { recursive: true, force: true });
  }
}
