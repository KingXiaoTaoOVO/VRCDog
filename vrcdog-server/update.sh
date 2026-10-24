#!/usr/bin/env bash
set -e

# ==============================================================
# VRCDog 一键/定时全自动更新脚本
# 作用: 自动检测版本更新，生产级编译 Web 前端，无损保留数据，重启后端并自愈 OpenResty 配置
# ==============================================================

# 确保环境变量包含常见的构建工具路径
export PATH="/root/.bun/bin:/usr/local/bin:/usr/bin:$PATH"

# 1. 并发文件锁保护，避免多个定时任务冲突
LOCKFILE="/tmp/vrcdog-update.lock"
exec 200>"$LOCKFILE"
if ! flock -n 200; then
    echo "[VRCDog] 另一个更新进程正在运行中，跳过本次执行。"
    exit 0
fi

# 2. 定位目录结构
REPO_DIR="/opt/VRCDog"
if [ ! -d "$REPO_DIR" ]; then
    if [ -d "../.git" ]; then
        REPO_DIR="$(cd .. && pwd)"
    elif [ -d ".git" ]; then
        REPO_DIR="$(pwd)"
    else
        REPO_DIR="/opt/VRCDog"
        mkdir -p "$REPO_DIR"
    fi
fi

SERVER_DIR="$REPO_DIR/vrcdog-server"
WEB_STATIC_DIR="/opt/1panel/apps/openresty/openresty/www/sites/vrcdog-web/index"
NGINX_CONF="/opt/1panel/apps/openresty/openresty/conf/conf.d/vrcdog-web.conf"

cd "$REPO_DIR"

# 3. 检查代码更新
FORCE_UPDATE=0
if [ "${1:-}" = "--force" ]; then
    FORCE_UPDATE=1
fi

LOCAL_HASH=""
REMOTE_HASH=""

if [ -d "$REPO_DIR/.git" ]; then
    git fetch origin main --quiet 2>/dev/null || true
    LOCAL_HASH=$(git rev-parse HEAD 2>/dev/null || echo "")
    REMOTE_HASH=$(git rev-parse origin/main 2>/dev/null || echo "")

    if [ "$FORCE_UPDATE" -eq 0 ] && [ -n "$LOCAL_HASH" ] && [ "$LOCAL_HASH" = "$REMOTE_HASH" ]; then
        # 远程无新提交且非强制更新，立即静默退出
        exit 0
    fi
fi

echo "=================================================="
echo " [VRCDog] 检测到代码更新或收到强制更新指令"
echo " 本地提交: ${LOCAL_HASH:-未知}"
echo " 远程提交: ${REMOTE_HASH:-未知}"
echo " 触发时间: $(date '+%Y-%m-%d %H:%M:%S')"
echo "=================================================="

# 4. 代码同步与拉取
echo "[1/5] 同步最新 Git 代码..."
if [ -d "$REPO_DIR/.git" ]; then
    git fetch --all --prune
    git reset --hard origin/main
    # 严格保留后端数据与配置文件
    git clean -fd -e vrcdog-server/data -e vrcdog-server/.env
else
    echo " -> 当前非 Git 仓库，从官方源下载最新包替换源码..."
    TMP_ARCHIVE="/tmp/vrcdog-latest.tar.gz"
    TMP_DIR="/tmp/vrcdog-extract-$$"
    mkdir -p "$TMP_DIR"

    if ! curl -fsSL --connect-timeout 8 "https://ghfast.top/https://github.com/KingXiaoTaoOVO/VRCDog/archive/refs/heads/main.tar.gz" -o "$TMP_ARCHIVE"; then
        echo " -> 高速代理超时，尝试直连 GitHub..."
        curl -fsSL --connect-timeout 15 "https://github.com/KingXiaoTaoOVO/VRCDog/archive/refs/heads/main.tar.gz" -o "$TMP_ARCHIVE"
    fi

    tar -xzf "$TMP_ARCHIVE" -C "$TMP_DIR"
    cp -rf "$TMP_DIR"/VRCDog-main/* "$REPO_DIR/"
    rm -rf "$TMP_ARCHIVE" "$TMP_DIR"
fi

# 5. 编译 Web 前端（VRCDOG_BASE=/web/）
echo "[2/5] 编译生产级 Web 前端应用 (VRCDOG_BASE=/web/)..."
cd "$REPO_DIR"
export VRCDOG_BASE=/web/

if command -v pnpm >/dev/null 2>&1; then
    pnpm install --frozen-lockfile || pnpm install
    pnpm run build:web || node scripts/build-web.mjs
elif command -v bun >/dev/null 2>&1; then
    bun install
    bun run build:web || node scripts/build-web.mjs
elif command -v npm >/dev/null 2>&1; then
    npm install
    npm run build:web || node scripts/build-web.mjs
else
    echo " -> 警告: 未找到 pnpm/bun/npm，尝试使用 Docker node 编译前端..."
    docker run --rm -v "$REPO_DIR":/app -w /app -e VRCDOG_BASE=/web/ node:22-alpine sh -c "corepack enable && pnpm install && pnpm run build:web"
fi

# 6. 同步前端静态资源到 1Panel 静态站点目录
echo "[3/5] 部署 Web 静态资源到 $WEB_STATIC_DIR ..."
if [ -d "$REPO_DIR/dist" ]; then
    mkdir -p "$WEB_STATIC_DIR"
    if command -v rsync >/dev/null 2>&1; then
        rsync -a --delete "$REPO_DIR/dist/" "$WEB_STATIC_DIR/"
    else
        rm -rf "$WEB_STATIC_DIR"/*
        cp -rf "$REPO_DIR/dist"/* "$WEB_STATIC_DIR/"
    fi
    chmod -R 755 "$WEB_STATIC_DIR" 2>/dev/null || true
    echo " -> Web 静态资源部署成功！"
else
    echo " -> 错误: 未生成 dist 目录，跳过静态文件同步。"
fi

# 7. 确保 OpenResty SPA 路由与缓存配置生效
if [ -f "$NGINX_CONF" ]; then
    if ! grep -q "try_files" "$NGINX_CONF"; then
        echo " -> 自动配置 OpenResty SPA 路由 fallback..."
        sed -i '/root \/www\/sites\/vrcdog-web\/index;/a \    location / {\n        try_files $uri $uri/ /index.html;\n    }\n    location /assets/ {\n        expires 30d;\n        add_header Cache-Control "public, no-transform";\n        try_files $uri =404;\n    }' "$NGINX_CONF"
    fi
fi

# 8. 校验数据持久化目录与权限，并重启后端容器
echo "[4/5] 校验持久化数据目录及权限并启动容器..."
cd "$SERVER_DIR"
mkdir -p "$SERVER_DIR/data"
chown -R 10001:10001 "$SERVER_DIR/data" 2>/dev/null || true
chmod -R 777 "$SERVER_DIR/data" 2>/dev/null || true
chcon -Rt svirt_sandbox_file_t "$SERVER_DIR/data" 2>/dev/null || true

docker compose up -d --build

# 平滑重新加载 OpenResty（热重载，零停机）
docker exec openresty nginx -s reload 2>/dev/null || true

# 9. 服务健康检查
echo "[5/5] 执行健康检查..."
sleep 3

if curl -fsSL --connect-timeout 5 http://127.0.0.1:21451/ping 2>/dev/null | grep -q "ok" || \
   curl -fsSL --connect-timeout 5 http://127.0.0.1:11451/ping 2>/dev/null | grep -q "ok"; then
    echo " -> 后端服务: 正常运行 (HTTP 200 /ping)"
else
    echo " -> 警告: 后端服务未在预期端口响应，查看最新日志:"
    docker compose logs --tail=15
fi

if curl -fsSL --connect-timeout 5 http://127.0.0.1:9999/index.html 2>/dev/null | grep -q "VrcDog"; then
    echo " -> Web 前端: 正常运行 (HTTP 200 /web)"
else
    echo " -> 警告: Web 前端静态站点响应异常"
fi

echo "=================================================="
echo " [VRCDog] 自动更新完成！当前版本: $(git rev-parse --short HEAD 2>/dev/null || echo '最新')"
echo "=================================================="
