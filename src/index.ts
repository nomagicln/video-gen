#!/usr/bin/env node
async function main(): Promise<void> {
  const { buildHeadlessProgram } = await import('./bin/headless.js');
  const program = buildHeadlessProgram();
  await program.parseAsync(process.argv);
}

main().catch((e) => {
  console.error(`[error] ${e instanceof Error ? e.message : String(e)}`);
  process.exit(1);
});
