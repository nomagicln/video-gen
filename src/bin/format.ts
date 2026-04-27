export function kv(rows: Array<[string, string]>): string {
  const w = Math.max(...rows.map(([k]) => k.length), 1);
  return rows.map(([k, v]) => `${k.padEnd(w, ' ')} : ${v}`).join('\n');
}
