import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { discover } from '../../src/pipeline/discover.js';

let root: string;

const touch = (p: string) => {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, '');
};

beforeEach(() => {
  root = fs.mkdtempSync(path.join(os.tmpdir(), 'vg-discover-'));
});
afterEach(() => {
  fs.rmSync(root, { recursive: true });
});

describe('discover', () => {
  it('pairs by basename in flat input dir', () => {
    touch(path.join(root, '01_a.jpg'));
    touch(path.join(root, '01_a.mp3'));
    touch(path.join(root, '02_b.png'));
    touch(path.join(root, '02_b.wav'));
    const r = discover(root);
    expect(r.pairs.map((p) => p.basename)).toEqual(['01_a', '02_b']);
    expect(r.orphans).toEqual([]);
  });

  it('also scans images/ and audio/ subdirs', () => {
    touch(path.join(root, 'images', '01_a.jpg'));
    touch(path.join(root, 'audio', '01_a.mp3'));
    const r = discover(root);
    expect(r.pairs.map((p) => p.basename)).toEqual(['01_a']);
  });

  it('mixes flat and subdir layouts in same run', () => {
    touch(path.join(root, '01_a.jpg'));
    touch(path.join(root, 'audio', '01_a.mp3'));
    touch(path.join(root, 'images', '02_b.png'));
    touch(path.join(root, '02_b.wav'));
    const r = discover(root);
    expect(r.pairs.map((p) => p.basename)).toEqual(['01_a', '02_b']);
  });

  it('reports image without audio as orphan', () => {
    touch(path.join(root, '01_a.jpg'));
    touch(path.join(root, '02_b.jpg'));
    touch(path.join(root, '02_b.mp3'));
    const r = discover(root);
    expect(r.pairs.map((p) => p.basename)).toEqual(['02_b']);
    expect(r.orphans).toEqual([
      { kind: 'image', basename: '01_a', path: path.join(root, '01_a.jpg') },
    ]);
  });

  it('reports audio without image as orphan', () => {
    touch(path.join(root, '01_a.mp3'));
    touch(path.join(root, '02_b.jpg'));
    touch(path.join(root, '02_b.mp3'));
    const r = discover(root);
    expect(r.orphans).toEqual([
      { kind: 'audio', basename: '01_a', path: path.join(root, '01_a.mp3') },
    ]);
  });

  it('throws on duplicate image basename', () => {
    touch(path.join(root, '01_a.jpg'));
    touch(path.join(root, 'images', '01_a.png'));
    touch(path.join(root, '01_a.mp3'));
    expect(() => discover(root)).toThrow(/ambiguous basename: 01_a/i);
  });

  it('throws on duplicate audio basename', () => {
    touch(path.join(root, '01_a.jpg'));
    touch(path.join(root, '01_a.mp3'));
    touch(path.join(root, 'audio', '01_a.wav'));
    expect(() => discover(root)).toThrow(/ambiguous basename: 01_a/i);
  });

  it('throws when no pair found at all', () => {
    touch(path.join(root, '01_a.jpg'));
    expect(() => discover(root)).toThrow(/no image\/audio pairs found/i);
  });

  it('throws when input dir does not exist', () => {
    expect(() => discover(path.join(root, 'missing'))).toThrow(/does not exist/i);
  });

  it('ignores unrelated extensions', () => {
    touch(path.join(root, '01_a.jpg'));
    touch(path.join(root, '01_a.mp3'));
    touch(path.join(root, 'README.txt'));
    touch(path.join(root, '01_a.aif'));
    const r = discover(root);
    expect(r.pairs.map((p) => p.basename)).toEqual(['01_a']);
  });

  it('sorts pairs by basename, lexicographic', () => {
    touch(path.join(root, '10_x.jpg'));
    touch(path.join(root, '10_x.mp3'));
    touch(path.join(root, '02_a.jpg'));
    touch(path.join(root, '02_a.mp3'));
    const r = discover(root);
    expect(r.pairs.map((p) => p.basename)).toEqual(['02_a', '10_x']);
  });
});
