import { createHash } from 'node:crypto';
import { createWriteStream } from 'node:fs';
import { mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { pipeline } from 'node:stream/promises';
import { Readable } from 'node:stream';
import path from 'node:path';
import process from 'node:process';

const ROOT = path.resolve(import.meta.dirname, '..');
const CACHE_DIR = path.join(ROOT, '.scratch', 'python-runtime');
const RUNTIME_DIR = path.join(ROOT, 'src-tauri', 'resources', 'python-runtime');
const PYTHON_VERSION = '3.13.12';
const PYTHON_ARCHIVE = `python-${PYTHON_VERSION}-embed-amd64.zip`;
const PYTHON_URL = `https://www.python.org/ftp/python/${PYTHON_VERSION}/${PYTHON_ARCHIVE}`;
const PYTHON_SHA256 = '76f238f606250c87c6beac75dccd35ee99070a13490555936abb6cb64ecce3d0';
const GET_PIP_URL = 'https://bootstrap.pypa.io/get-pip.py';
const GET_PIP_SHA256 = '25b5c39ade96bab5eabe6404ce83cab6da2deb5fe3c07d9881f43803edb6f9c8';
const PIP_VERSION = '26.2';
const WINDOWS_TAR = path.join(process.env.SystemRoot || 'C:\\Windows', 'System32', 'tar.exe');

const run = (command, args, options = {}) => new Promise((resolve, reject) => {
  const child = spawn(command, args, { stdio: 'inherit', windowsHide: true, ...options });
  child.once('error', reject);
  child.once('exit', (code) => code === 0 ? resolve() : reject(new Error(`${command} exited with code ${code}`)));
});

const sha256 = async (file) => createHash('sha256').update(await readFile(file)).digest('hex');

async function download(url, target, expectedHash) {
  try {
    if (await sha256(target) === expectedHash) return;
  } catch {}
  const response = await fetch(url, { redirect: 'follow' });
  if (!response.ok || !response.body) throw new Error(`Download failed: ${url} (${response.status})`);
  await pipeline(Readable.fromWeb(response.body), createWriteStream(target));
  const actual = await sha256(target);
  if (actual !== expectedHash) throw new Error(`SHA-256 mismatch for ${path.basename(target)}: ${actual}`);
}

async function prepare() {
  if (process.platform !== 'win32') {
    console.log('Skipping embedded Python runtime preparation outside Windows.');
    return;
  }
  await mkdir(CACHE_DIR, { recursive: true });
  const archive = path.join(CACHE_DIR, PYTHON_ARCHIVE);
  const getPip = path.join(CACHE_DIR, 'get-pip.py');
  await download(PYTHON_URL, archive, PYTHON_SHA256);
  await download(GET_PIP_URL, getPip, GET_PIP_SHA256);

  const stagedRuntime = `${RUNTIME_DIR}.next-${process.pid}`;
  const previousRuntime = `${RUNTIME_DIR}.previous-${process.pid}`;
  await rm(stagedRuntime, { recursive: true, force: true });
  await rm(previousRuntime, { recursive: true, force: true });
  await mkdir(stagedRuntime, { recursive: true });
  await run(WINDOWS_TAR, ['-xf', archive, '-C', stagedRuntime]);

  const pthFile = path.join(stagedRuntime, 'python313._pth');
  const pth = await readFile(pthFile, 'utf8');
  await writeFile(pthFile, pth.replace('#import site', 'import site'), 'utf8');

  const python = path.join(stagedRuntime, 'python.exe');
  const isolatedEnv = { ...process.env, PYTHONNOUSERSITE: '1', PYTHONUTF8: '1' };
  await run(python, [getPip, '--disable-pip-version-check', '--no-warn-script-location', `pip==${PIP_VERSION}`], { env: isolatedEnv });
  await run(python, [
    '-m', 'pip', 'install', '--disable-pip-version-check', '--no-warn-script-location',
    '--no-cache-dir', '--only-binary=:all:', '--no-compile',
    'requests==2.32.5',
    'beautifulsoup4==4.14.2',
    'certifi==2026.5.20',
    'soupsieve==2.8.4',
    'charset-normalizer==3.4.9',
    'idna==3.18',
    'urllib3==2.7.0',
    'typing-extensions==4.16.0',
  ], { env: isolatedEnv });
  await run(python, ['-I', '-c', 'import requests, bs4, certifi, json; print(json.dumps({"ok": True}))'], { env: isolatedEnv });

  await writeFile(path.join(stagedRuntime, 'vrcdog-runtime.json'), JSON.stringify({
    pythonVersion: PYTHON_VERSION,
    architecture: 'windows-x64',
    packages: [
      'requests==2.32.5',
      'beautifulsoup4==4.14.2',
      'certifi==2026.5.20',
      'soupsieve==2.8.4',
      'charset-normalizer==3.4.9',
      'idna==3.18',
      'urllib3==2.7.0',
      'typing-extensions==4.16.0',
    ],
    source: PYTHON_URL,
    archiveSha256: PYTHON_SHA256,
  }, null, 2));
  let movedPrevious = false;
  try {
    await rename(RUNTIME_DIR, previousRuntime);
    movedPrevious = true;
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  try {
    await rename(stagedRuntime, RUNTIME_DIR);
  } catch (error) {
    if (movedPrevious) await rename(previousRuntime, RUNTIME_DIR);
    throw error;
  }
  if (movedPrevious) {
    console.warn(`Previous embedded runtime retained for deferred cleanup: ${previousRuntime}`);
  }
  console.log(`Embedded Python runtime ready: ${RUNTIME_DIR}`);
}

prepare().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
