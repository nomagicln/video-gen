import fs from 'node:fs';
import path from 'node:path';

export type Tool = 'ffmpeg' | 'ffprobe';

const ENV_VAR: Record<Tool, string> = {
  ffmpeg: 'VIDEO_GEN_FFMPEG',
  ffprobe: 'VIDEO_GEN_FFPROBE',
};

const exeName = (name: string) => (process.platform === 'win32' ? `${name}.exe` : name);

export type ResolveOptions = {
  execDir?: string;
};

export function resolveBinary(tool: Tool, opts: ResolveOptions = {}): string {
  const override = process.env[ENV_VAR[tool]];
  if (override && override.trim()) return override.trim();

  const execDir = opts.execDir ?? path.dirname(process.execPath);
  const sibling = path.join(execDir, exeName(tool));
  try {
    if (fs.existsSync(sibling)) return sibling;
  } catch {
    /* ignore */
  }

  return exeName(tool);
}
