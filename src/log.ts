export type LogMode = 'text' | 'quiet' | 'json';

export type SegmentEvent = {
  index: number;
  total: number;
  name: string;
  kind: 'lead-in' | 'unit' | 'gap' | 'tail';
  basename?: string;
  durationMs: number;
  ms: number;
  status?: 'ok' | 'fail';
};

export type Logger = {
  discover(p: { units: number; orphans: number }): void;
  plan(p: { segments: number; totalMs: number; summary: string }): void;
  warn(message: string): void;
  segment(e: SegmentEvent): void;
  concat(p: { output: string; bytes: number; durationMs: number }): void;
  done(p: { output: string; bytes: number; durationMs: number; ms: number }): void;
  error(message: string, code?: number): void;
};

const formatBytes = (b: number) => {
  if (b < 1024) return `${b}B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)}KB`;
  return `${(b / 1024 / 1024).toFixed(1)}MB`;
};
const fmtSec = (ms: number) => `${(ms / 1000).toFixed(1)}s`;

function writeStdout(s: string): void {
  process.stdout.write(s);
}
function writeStderr(s: string): void {
  process.stderr.write(s);
}

export function createLogger(opts: { mode: LogMode }): Logger {
  const { mode } = opts;

  return {
    discover(p) {
      if (mode === 'quiet') return;
      if (mode === 'json') {
        writeStdout(JSON.stringify({ phase: 'discover', units: p.units, orphans: p.orphans }) + '\n');
      } else {
        writeStdout(`[discover] ${p.units} units (${p.orphans} orphans skipped)\n`);
      }
    },

    plan(p) {
      if (mode === 'quiet') return;
      if (mode === 'json') {
        writeStdout(JSON.stringify({ phase: 'plan', segments: p.segments, total_ms: p.totalMs }) + '\n');
      } else {
        writeStdout(`[plan]     ${p.segments} segments (${p.summary}) total ${fmtSec(p.totalMs)}\n`);
      }
    },

    warn(message) {
      if (mode === 'json') {
        writeStderr(JSON.stringify({ phase: 'warn', message }) + '\n');
      } else {
        writeStderr(`WARN ${message}\n`);
      }
    },

    segment(e) {
      if (mode === 'quiet') return;
      if (mode === 'json') {
        const obj: Record<string, unknown> = {
          phase: 'build', seg: e.name, kind: e.kind,
          duration_ms: e.durationMs, status: e.status ?? 'ok', ms: e.ms,
        };
        if (e.basename) obj.basename = e.basename;
        writeStdout(JSON.stringify(obj) + '\n');
      } else {
        const idx = `[${String(e.index).padStart(2, ' ')}/${e.total}]`;
        const label = e.kind === 'unit' ? `unit ${e.basename ?? ''}` : e.kind;
        writeStdout(
          `[build]    ${idx} ${e.name.padEnd(8, ' ')} ${label.padEnd(20, ' ')} ${(e.durationMs / 1000).toFixed(3)}s ... ${e.status ?? 'ok'} (${fmtSec(e.ms)})\n`,
        );
      }
    },

    concat(p) {
      if (mode === 'quiet') return;
      if (mode === 'json') {
        writeStdout(JSON.stringify({ phase: 'concat', output: p.output, bytes: p.bytes, duration_ms: p.durationMs }) + '\n');
      } else {
        writeStdout(`[concat]   ${p.output} (${formatBytes(p.bytes)}, ${fmtSec(p.durationMs)})\n`);
      }
    },

    done(p) {
      if (mode === 'json') {
        writeStdout(JSON.stringify({ phase: 'done', output: p.output, bytes: p.bytes, duration_ms: p.durationMs, ms: p.ms }) + '\n');
      } else if (mode === 'quiet') {
        writeStdout(`done: ${p.output} (${formatBytes(p.bytes)}, ${fmtSec(p.durationMs)})\n`);
      } else {
        writeStdout(`done in ${fmtSec(p.ms)}\n`);
      }
    },

    error(message, code = 1) {
      if (mode === 'json') {
        writeStderr(JSON.stringify({ phase: 'error', code, message }) + '\n');
      } else {
        writeStderr(`[error] ${message}\n`);
      }
    },
  };
}
