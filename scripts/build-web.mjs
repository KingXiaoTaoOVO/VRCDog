#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import process from 'node:process';

process.env.VRCDOG_BASE = process.env.VRCDOG_BASE || '/web/';

console.log(`[VRCDog] Building web frontend with VRCDOG_BASE="${process.env.VRCDOG_BASE}"...`);

const isWin = process.platform === 'win32';
const shellCmd = isWin ? 'cmd.exe' : '/bin/sh';
const shellArgs = isWin
  ? ['/d', '/s', '/c', 'bun run build || pnpm run build || npm run build']
  : ['-c', 'bun run build || pnpm run build || npm run build'];

const result = spawnSync(shellCmd, shellArgs, {
  stdio: 'inherit',
  env: process.env,
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}

console.log('[VRCDog] Web frontend build complete!');
