#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import process from 'node:process';

process.env.VRCDOG_BASE = process.env.VRCDOG_BASE || '/web/';

console.log(`[VRCDog] Building web frontend with VRCDOG_BASE="${process.env.VRCDOG_BASE}"...`);

const isWin = process.platform === 'win32';

// Detect package manager cleanly without noisy shell errors
let packageManager = 'npm';
try {
  if (spawnSync(isWin ? 'pnpm.cmd' : 'pnpm', ['--version'], { stdio: 'ignore' }).status === 0) {
    packageManager = 'pnpm';
  } else if (spawnSync(isWin ? 'bun.exe' : 'bun', ['--version'], { stdio: 'ignore' }).status === 0) {
    packageManager = 'bun';
  }
} catch {
  packageManager = 'npm';
}

console.log(`[VRCDog] Using package manager: ${packageManager}`);

const shellCmd = isWin ? 'cmd.exe' : '/bin/sh';
const shellArgs = isWin
  ? ['/d', '/s', '/c', `${packageManager} run build`]
  : ['-c', `${packageManager} run build`];

const result = spawnSync(shellCmd, shellArgs, {
  stdio: 'inherit',
  env: process.env,
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

console.log('[VRCDog] Web frontend build complete!');
