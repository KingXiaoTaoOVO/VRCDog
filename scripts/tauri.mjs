#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { delimiter, join } from "node:path";
import { fileURLToPath } from "node:url";

// Silence the Node 22.11+ DEP0205 deprecation ("module.register() is
// deprecated. Use module.registerHooks() instead.") that is emitted from
// inside the native @tauri-apps/cli binding. The filter runs before the
// CLI is required below so the warning never reaches stderr. Other
// deprecation warnings are preserved.
const _originalEmitWarning = process.emitWarning.bind(process);
process.emitWarning = function emitWarning(warning, ...rest) {
  const code =
    warning && typeof warning === "object"
      ? warning.code
      : rest[1]; // legacy signature: (warning, type, code, ctor)
  if (code === "DEP0205") return;
  return _originalEmitWarning(warning, ...rest);
};

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const require = createRequire(import.meta.url);
const cargoHome = process.env.CARGO_HOME
  ?? (process.env.USERPROFILE ? `${process.env.USERPROFILE}\\.cargo` : undefined);

if (cargoHome) {
  const cargoBin = `${cargoHome}\\bin`;
  const currentPath = process.env.PATH ?? "";
  const entries = currentPath.split(delimiter);
  if (!entries.some((entry) => entry.toLowerCase() === cargoBin.toLowerCase())) {
    process.env.PATH = `${cargoBin}${delimiter}${currentPath}`;
  }
}

const args = process.argv.slice(2);
const command = args[0];
const isWindowsTauriRun = process.platform === "win32"
  && (command === "dev" || command === "build");

if (isWindowsTauriRun) {
  const setupScript = join(projectRoot, "src-tauri", "scripts", "download_ffmpeg.ps1");
  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", setupScript],
    {
      cwd: projectRoot,
      env: process.env,
      stdio: "inherit",
      windowsHide: true,
    },
  );
  if (result.error || result.status !== 0) {
    console.error("Unable to prepare the FFmpeg sidecar required by Tauri.");
    process.exit(result.status ?? 1);
  }
}

const { run } = require("@tauri-apps/cli");
await run(args, "vrcdog");
