export type RuntimeMode = 'mock' | 'tauri';

export function parseRuntimeModeFromArgs(args: string[]): RuntimeMode {
  const runtimeArg = args.find((arg) => arg.startsWith('--runtime='));
  const runtime = runtimeArg?.split('=', 2)[1];

  return runtime === 'mock' ? 'mock' : 'tauri';
}
