import { describe, it, expect } from 'vitest';
import { plan, type Unit } from '../../src/pipeline/plan.js';

const u = (basename: string, durationMs: number): Unit => ({
  basename,
  imagePath: `/img/${basename}.jpg`,
  audioPath: `/aud/${basename}.mp3`,
  audioDurationMs: durationMs,
});

describe('plan', () => {
  it('throws on empty units', () => {
    expect(() => plan([], { leadInMs: 0, tailMs: 0, gapMs: 0 })).toThrow(/at least one unit/i);
  });

  it('one unit, no padding → one segment', () => {
    const segs = plan([u('a', 1000)], { leadInMs: 0, tailMs: 0, gapMs: 0 });
    expect(segs).toEqual([
      { kind: 'unit', image: '/img/a.jpg', audio: '/aud/a.mp3', durationMs: 1000 },
    ]);
  });

  it('skips zero-duration lead-in / tail / gap', () => {
    const segs = plan([u('a', 1000), u('b', 2000)], { leadInMs: 0, tailMs: 0, gapMs: 0 });
    expect(segs.map((s) => s.kind)).toEqual(['unit', 'unit']);
  });

  it('three units with full padding', () => {
    const segs = plan(
      [u('a', 1000), u('b', 2000), u('c', 3000)],
      { leadInMs: 2000, tailMs: 1000, gapMs: 500 },
    );
    expect(segs).toEqual([
      { kind: 'lead-in', image: '/img/a.jpg', durationMs: 2000 },
      { kind: 'unit', image: '/img/a.jpg', audio: '/aud/a.mp3', durationMs: 1000 },
      { kind: 'gap', image: '/img/a.jpg', durationMs: 500 },
      { kind: 'unit', image: '/img/b.jpg', audio: '/aud/b.mp3', durationMs: 2000 },
      { kind: 'gap', image: '/img/b.jpg', durationMs: 500 },
      { kind: 'unit', image: '/img/c.jpg', audio: '/aud/c.mp3', durationMs: 3000 },
      { kind: 'tail', image: '/img/c.jpg', durationMs: 1000 },
    ]);
  });

  it('lead-in shows first image, tail shows last', () => {
    const segs = plan([u('a', 1000), u('b', 1000)], { leadInMs: 500, tailMs: 500, gapMs: 0 });
    expect(segs[0]).toMatchObject({ kind: 'lead-in', image: '/img/a.jpg' });
    expect(segs[segs.length - 1]).toMatchObject({ kind: 'tail', image: '/img/b.jpg' });
  });

  it('gap shows preceding unit image, not next', () => {
    const segs = plan([u('a', 1000), u('b', 1000)], { leadInMs: 0, tailMs: 0, gapMs: 300 });
    const gap = segs.find((s) => s.kind === 'gap');
    expect(gap).toMatchObject({ image: '/img/a.jpg', durationMs: 300 });
  });

  it('rejects negative durations', () => {
    expect(() => plan([u('a', 1000)], { leadInMs: -1, tailMs: 0, gapMs: 0 })).toThrow(/negative/i);
    expect(() => plan([u('a', 1000)], { leadInMs: 0, tailMs: -1, gapMs: 0 })).toThrow(/negative/i);
    expect(() => plan([u('a', 1000)], { leadInMs: 0, tailMs: 0, gapMs: -1 })).toThrow(/negative/i);
  });
});
