import type { RuntimeMode } from './runtimeMode';

export interface DevLaunchConfig {
  runtimeMode: RuntimeMode;
  viteArgs: string[];
}

export function buildDevLaunchConfig(argv: string[]): DevLaunchConfig {
  const runtimeArg = argv.find((arg) => arg.startsWith('--runtime='));
  const runtimeMode = runtimeArg?.split('=', 2)[1] === 'mock' ? 'mock' : 'tauri';
  const viteArgs = ['--port=5173', '--host=0.0.0.0', ...argv.slice(2).filter((arg) => !arg.startsWith('--runtime='))];

  return { runtimeMode, viteArgs };
}
