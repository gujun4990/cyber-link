import { spawn } from 'node:child_process';

import { buildDevLaunchConfig } from '../src/devLauncher.ts';

const config = buildDevLaunchConfig(process.argv);

const child = spawn('vite', config.viteArgs, {
  stdio: 'inherit',
  env: {
    ...process.env,
    VITE_CYBER_LINK_RUNTIME: config.runtimeMode,
  },
  shell: false,
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 0);
});
