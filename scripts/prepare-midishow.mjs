#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { platform } from "node:os";
import { fileURLToPath } from "node:url";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const midishowRoot = fileURLToPath(new URL("../src-python/midishow-downloader", import.meta.url));
const userAgent = process.env.npm_config_user_agent?.toLowerCase() ?? "";
const packageManager = userAgent.startsWith("bun/") ? "bun" : "pnpm";
const args = packageManager === "bun"
  ? ["install", "--frozen-lockfile", "--production", "--ignore-scripts"]
  : ["install", "--frozen-lockfile", "--prod", "--ignore-scripts", "--ignore-workspace"];

const result = spawnSync(packageManager, args, {
  cwd: midishowRoot,
  env: process.env,
  stdio: "inherit",
  shell: platform() === "win32",
  windowsHide: true,
});

if (result.error) {
  console.error(`Unable to prepare MIDIShow with ${packageManager}: ${result.error.message}`);
  process.exit(1);
}

if (result.status !== 0) {
  console.error(`MIDIShow dependency installation failed in ${midishowRoot}.`);
  console.error(`Run this command from ${projectRoot} with pnpm or Bun installed.`);
  process.exit(result.status ?? 1);
}
