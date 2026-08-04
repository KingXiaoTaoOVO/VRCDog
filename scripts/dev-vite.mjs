#!/usr/bin/env node

import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const configFile = fileURLToPath(new URL("../vite.config.ts", import.meta.url));

// Free the fixed Tauri development port before Vite starts listening. Importing
// this module performs the cleanup on Windows, macOS, and Linux.
await import("./kill-port-1420.mjs");

const server = await createServer({
  root: projectRoot,
  configFile,
});

await server.listen();
server.printUrls();
server.bindCLIShortcuts({ print: true });

let closing = false;
const close = async () => {
  if (closing) return;
  closing = true;
  await server.close();
  process.exit(0);
};

process.once("SIGINT", close);
process.once("SIGTERM", close);
