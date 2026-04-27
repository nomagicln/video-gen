import { Command } from 'commander';

export function buildHeadlessProgram(): Command {
  const program = new Command('video-gen');
  program.command('build').description('build mp4 from input dir').action(() => {
    throw new Error('not implemented yet');
  });
  return program;
}
