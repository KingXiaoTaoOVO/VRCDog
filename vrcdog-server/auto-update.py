#!/usr/bin/env python3
import os
import sys
import shutil
import tarfile
import urllib.request
import subprocess
import time

REPO_DIR = '/opt/VRCDog'
SERVER_DIR = os.path.join(REPO_DIR, 'vrcdog-server')
WEB_STATIC_DIR = '/opt/1panel/apps/openresty/openresty/www/sites/vrcdog-web/index'
NGINX_CONF = '/opt/1panel/apps/openresty/openresty/conf/conf.d/vrcdog-web.conf'
ETAG_FILE = os.path.join(REPO_DIR, '.last_etag')
ARCHIVE_URL = 'https://ghfast.top/https://github.com/KingXiaoTaoOVO/VRCDog/archive/refs/heads/main.tar.gz'
ARCHIVE_URL_BACKUP = 'https://ghproxy.net/https://github.com/KingXiaoTaoOVO/VRCDog/archive/refs/heads/main.tar.gz'

# 1. 进程锁保护
PID_FILE = '/tmp/vrcdog-update.pid'
if os.path.exists(PID_FILE):
    try:
        with open(PID_FILE, 'r') as f:
            old_pid = int(f.read().strip())
        os.kill(old_pid, 0)
        print(f'[VRCDog] 另一个更新进程 (PID: {old_pid}) 正在运行中，跳过本次执行。')
        sys.exit(0)
    except (ValueError, OSError):
        pass

with open(PID_FILE, 'w') as f:
    f.write(str(os.getpid()))

def cleanup():
    if os.path.exists(PID_FILE):
        try:
            os.remove(PID_FILE)
        except OSError:
            pass

import atexit
atexit.register(cleanup)

# 2. 检查 ETag 版本
force = '--force' in sys.argv
last_etag = ''
if os.path.exists(ETAG_FILE):
    try:
        with open(ETAG_FILE, 'r') as f:
            last_etag = f.read().strip()
    except Exception:
        pass

current_etag = ''
active_archive_url = ARCHIVE_URL
print('==================================================')
print(' [VRCDog] 正在检查最新版本更新...')
req = urllib.request.Request(ARCHIVE_URL, headers={'User-Agent': 'curl/7.88.1'})
try:
    with urllib.request.urlopen(req, timeout=10) as resp:
        current_etag = resp.headers.get('ETag', '').strip('"')
except Exception as e:
    print('检查主代理失败，尝试备用代理:', e)
    try:
        req = urllib.request.Request(ARCHIVE_URL_BACKUP, headers={'User-Agent': 'curl/7.88.1'})
        with urllib.request.urlopen(req, timeout=10) as resp:
            current_etag = resp.headers.get('ETag', '').strip('"')
            active_archive_url = ARCHIVE_URL_BACKUP
    except Exception as e2:
        print('检查备用代理失败:', e2)
        if not force:
            sys.exit(1)

print(f' 本地 ETag: {last_etag or "无记录"}')
print(f' 远程 ETag: {current_etag or "未知"}')

if not force and last_etag and current_etag and last_etag == current_etag:
    print(' 当前已是最新版本，无需更新。')
    sys.exit(0)

print(' 检测到新版本！开始全自动更新流程...')
print('==================================================')

# 3. 高速下载并解压源码包
tmp_archive = '/tmp/vrcdog-main.tar.gz'
tmp_extract = '/tmp/vrcdog-extract'

print('[1/5] 高速下载最新源码包并替换更新...')
if os.path.exists(tmp_extract):
    shutil.rmtree(tmp_extract, ignore_errors=True)
os.makedirs(tmp_extract, exist_ok=True)

with urllib.request.urlopen(req, timeout=60) as r, open(tmp_archive, 'wb') as f:
    shutil.copyfileobj(r, f)

print(' -> 解压最新源码包...')
with tarfile.open(tmp_archive, 'r:gz') as tar:
    tar.extractall(tmp_extract)

extracted_root = None
for name in os.listdir(tmp_extract):
    candidate = os.path.join(tmp_extract, name)
    if os.path.isdir(candidate) and 'VRCDog' in name:
        extracted_root = candidate
        break

if not extracted_root:
    print('错误: 解压后未找到源码目录')
    sys.exit(1)

# 严格保留后端数据目录与 .env 配置文件
os.makedirs(REPO_DIR, exist_ok=True)
preserve = {
    os.path.normpath(os.path.join(SERVER_DIR, 'data')),
    os.path.normpath(os.path.join(SERVER_DIR, '.env')),
}

for root, dirs, files in os.walk(extracted_root):
    rel_path = os.path.relpath(root, extracted_root)
    dest_dir = os.path.normpath(os.path.join(REPO_DIR, rel_path))
    os.makedirs(dest_dir, exist_ok=True)
    for file in files:
        src_file = os.path.join(root, file)
        dest_file = os.path.normpath(os.path.join(dest_dir, file))
        if any(dest_file.startswith(p) for p in preserve):
            continue
        shutil.copy2(src_file, dest_file)

if current_etag:
    with open(ETAG_FILE, 'w') as f:
        f.write(current_etag)

shutil.rmtree(tmp_extract, ignore_errors=True)
if os.path.exists(tmp_archive):
    os.remove(tmp_archive)

# 4. 编译生产级 Web 前端
print('[2/5] 编译生产级 Web 前端应用 (VRCDOG_BASE=/web/)...')
env = os.environ.copy()
env['HOME'] = '/root'
env['PATH'] = f'/usr/local/bin:/usr/bin:/opt/node/bin:{env.get("PATH", "")}'
env['VRCDOG_BASE'] = '/web/'

pnpm_bin = shutil.which('pnpm', path=env['PATH']) or '/usr/local/bin/pnpm'
npm_bin = shutil.which('npm', path=env['PATH']) or '/usr/local/bin/npm'

if os.path.exists(pnpm_bin):
    subprocess.run([pnpm_bin, 'config', 'set', 'registry', 'https://mirrors.cloud.tencent.com/npm/'], cwd=REPO_DIR, env=env, check=False)
    subprocess.run([pnpm_bin, 'install', '--frozen-lockfile'], cwd=REPO_DIR, env=env, check=False)
    subprocess.run(['node', 'scripts/build-web.mjs'], cwd=REPO_DIR, env=env, check=True)
elif os.path.exists(npm_bin):
    subprocess.run([npm_bin, 'config', 'set', 'registry', 'https://mirrors.cloud.tencent.com/npm/'], cwd=REPO_DIR, env=env, check=False)
    subprocess.run([npm_bin, 'install'], cwd=REPO_DIR, env=env, check=False)
    subprocess.run(['node', 'scripts/build-web.mjs'], cwd=REPO_DIR, env=env, check=True)
else:
    print('错误: 未找到 Node/Pnpm 构建工具')
    sys.exit(1)

# 5. 部署 Web 静态资源到 OpenResty
print(f'[3/5] 部署 Web 静态资源到 {WEB_STATIC_DIR} ...')
dist_dir = os.path.join(REPO_DIR, 'dist')
if os.path.isdir(dist_dir):
    os.makedirs(WEB_STATIC_DIR, exist_ok=True)
    rsync_bin = shutil.which('rsync', path=env['PATH'])
    if rsync_bin:
        subprocess.run([rsync_bin, '-a', '--delete', f'{dist_dir}/', f'{WEB_STATIC_DIR}/'], check=True)
    else:
        for f in os.listdir(WEB_STATIC_DIR):
            p = os.path.join(WEB_STATIC_DIR, f)
            if os.path.isdir(p):
                shutil.rmtree(p)
            else:
                os.remove(p)
        for f in os.listdir(dist_dir):
            src = os.path.join(dist_dir, f)
            dst = os.path.join(WEB_STATIC_DIR, f)
            if os.path.isdir(src):
                shutil.copytree(src, dst)
            else:
                shutil.copy2(src, dst)
    subprocess.run(['chmod', '-R', '755', WEB_STATIC_DIR], check=False)
    print(' -> Web 静态资源部署成功！')
else:
    print('错误: dist 目录不存在')
    sys.exit(1)

# 6. 确保 OpenResty SPA 路由与缓存配置生效
if os.path.exists(NGINX_CONF):
    with open(NGINX_CONF, 'r', encoding='utf-8') as f:
        conf_content = f.read()
    if 'try_files' not in conf_content:
        print(' -> 配置 OpenResty SPA fallback try_files...')
        spa_rule = """
    location / {
        try_files $uri $uri/ /index.html;
    }
    location /assets/ {
        expires 30d;
        add_header Cache-Control "public, no-transform";
        try_files $uri =404;
    }
"""
        target = 'root /www/sites/vrcdog-web/index;'
        if target in conf_content:
            conf_content = conf_content.replace(target, target + '\n' + spa_rule)
            with open(NGINX_CONF, 'w', encoding='utf-8') as f:
                f.write(conf_content)

# 7. 校验数据目录并重启后端容器
print('[4/5] 校验持久化数据目录及权限并启动容器...')
data_dir = os.path.join(SERVER_DIR, 'data')
os.makedirs(data_dir, exist_ok=True)
subprocess.run(['chown', '-R', '10001:10001', data_dir], check=False)
subprocess.run(['chmod', '-R', '777', data_dir], check=False)

subprocess.run(['docker', 'compose', 'up', '-d', '--build'], cwd=SERVER_DIR, env=env, check=True)

# 平滑重新加载 OpenResty（热重载，零停机）
subprocess.run(['docker', 'exec', 'openresty', 'nginx', '-s', 'reload'], check=False)

# 8. 执行健康检查
print('[5/5] 执行健康检查...')
time.sleep(3)

try:
    with urllib.request.urlopen('http://127.0.0.1:21451/ping', timeout=5) as r:
        if b'ok' in r.read():
            print(' -> 后端服务: 正常运行 (HTTP 200 /ping)')
        else:
            print(' -> 警告: 后端响应未包含 ok')
except Exception as e:
    print(' -> 警告: 后端健康检查异常:', e)

try:
    with urllib.request.urlopen('http://127.0.0.1:9999/index.html', timeout=5) as r:
        content = r.read().decode('utf-8', errors='ignore')
        if 'VrcDog' in content:
            print(' -> Web 前端: 正常运行 (HTTP 200 /web)')
        else:
            print(' -> 警告: Web 前端内容异常')
except Exception as e:
    print(' -> 警告: Web 前端健康检查异常:', e)

print('==================================================')
print(' [VRCDog] 自动更新完成！')
print('==================================================')
