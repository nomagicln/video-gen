export type Unit = {
  basename: string;
  imagePath: string;
  audioPath: string;
  audioDurationMs: number;
};

export type PlanOptions = {
  leadInMs: number;
  tailMs: number;
  gapMs: number;
};

export type Segment =
  | { kind: 'lead-in'; image: string; durationMs: number }
  | { kind: 'unit'; image: string; audio: string; durationMs: number }
  | { kind: 'gap'; image: string; durationMs: number }
  | { kind: 'tail'; image: string; durationMs: number };

export function plan(units: Unit[], opts: PlanOptions): Segment[] {
  if (units.length === 0) throw new Error('plan: requires at least one unit');
  if (opts.leadInMs < 0 || opts.tailMs < 0 || opts.gapMs < 0) {
    throw new Error('plan: negative duration not allowed');
  }

  const segs: Segment[] = [];
  if (opts.leadInMs > 0) {
    segs.push({ kind: 'lead-in', image: units[0]!.imagePath, durationMs: opts.leadInMs });
  }
  for (let i = 0; i < units.length; i++) {
    const u = units[i]!;
    segs.push({ kind: 'unit', image: u.imagePath, audio: u.audioPath, durationMs: u.audioDurationMs });
    const isLast = i === units.length - 1;
    if (!isLast && opts.gapMs > 0) {
      segs.push({ kind: 'gap', image: u.imagePath, durationMs: opts.gapMs });
    }
  }
  if (opts.tailMs > 0) {
    segs.push({ kind: 'tail', image: units[units.length - 1]!.imagePath, durationMs: opts.tailMs });
  }
  return segs;
}
