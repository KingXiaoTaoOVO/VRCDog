#!/usr/bin/env node
/**
 * Kill only the process that is listening on Vite's dev-server port.
 *
 * Do not kill every netstat row containing ":1420": established browser/WebView
 * client connections also contain that port and killing them causes
 * ERR_NETWORK_CHANGED while the app is loading modules or locale JSON files.
 */
import { execSync } from "node:child_process";
import { platform } from "node:os";

const PORT = 1420;

function killPid(pid) {
  try {
    if (platform() === "win32") {
      execSync(`taskkill /F /PID ${pid}`, {
        encoding: "utf8",
        timeout: 3000,
        stdio: "pipe",
      });
    } else {
      execSync(`kill -9 ${pid}`, {
        encoding: "utf8",
        timeout: 3000,
        stdio: "pipe",
      });
    }
    console.log(`Killed PID ${pid} (was listening on port ${PORT})`);
  } catch {
    // Process may have already exited.
  }
}

function killProcessOnWindows() {
  try {
    const stdout = execSync("netstat -ano -p tcp", {
      encoding: "utf8",
      timeout: 3000,
    });
    const pids = new Set();

    for (const line of stdout.trim().split(/\r?\n/)) {
      const parts = line.trim().split(/\s+/);
      if (parts[0] !== "TCP" || parts.length < 5) continue;

      const localAddress = parts[1];
      const state = parts[3];
      const pid = parts[4];

      if (localAddress.endsWith(`:${PORT}`) && state === "LISTENING" && pid && pid !== "0") {
        pids.add(pid);
      }
    }

    for (const pid of pids) {
      killPid(pid);
    }
  } catch {
    // No process found on that port; nothing to kill.
  }
}

function killProcessOnUnix() {
  try {
    const stdout = execSync(`lsof -tiTCP:${PORT} -sTCP:LISTEN 2>/dev/null`, {
      encoding: "utf8",
      timeout: 3000,
    });
    const pids = new Set(stdout.trim().split(/\r?\n/).filter(Boolean));

    for (const pid of pids) {
      killPid(pid);
    }
  } catch {
    // Nothing listening on that port.
  }
}

if (platform() === "win32") {
  killProcessOnWindows();
} else {
  killProcessOnUnix();
}
