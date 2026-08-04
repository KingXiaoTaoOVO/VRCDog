#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { platform } from "node:os";

const scripts = process.argv.slice(2);

if (scripts.length === 0) {
  console.error("Usage: node scripts/run-package-script.mjs <script> [...scripts]");
  process.exit(1);
}

const userAgent = process.env.npm_config_user_agent?.toLowerCase() ?? "";
const packageManager = userAgent.startsWith("bun/") ? "bun" : "pnpm";

for (const script of scripts) {
  const result = spawnSync(packageManager, ["run", script], {
    cwd: process.cwd(),
    env: process.env,
    stdio: "inherit",
    shell: platform() === "win32",
    windowsHide: true,
  });

  if (result.error) {
    console.error(`Unable to run ${script} with ${packageManager}: ${result.error.message}`);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
