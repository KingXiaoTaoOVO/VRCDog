#!/usr/bin/env bash
set -e

# ==============================================================
# VRCDog-Server 一键更新脚本
# 作用: 自动清理并替换旧版本源码，拉取最新代码，无损保留数据并自动重构启动
# ==============================================================

TARGET_DIR="/opt/VRCDog/vrcdog-server"
if [ ! -d "$TARGET_DIR" ]; then
    # 如果当前就在 vrcdog-server 目录下执行，则使用当前目录
    if [ -f "./Cargo.toml" ] && [ -f "./docker-compose.yml" ]; then
        TARGET_DIR="$(pwd)"
    else
        TARGET_DIR="/opt/VRCDog/vrcdog-server"
        mkdir -p "$TARGET_DIR"
    fi
fi

cd "$TARGET_DIR"

echo "=================================================="
echo " [VRCDog-Server] 开始一键在线更新..."
echo " 目标目录: $TARGET_DIR"
echo "=================================================="

# 1. 停止现有容器
echo "[1/4] 停止运行中的旧容器..."
docker compose down 2>/dev/null || true

# 2. 拉取/下载最新代码
echo "[2/4] 拉取最新源码并清理旧文件..."
PARENT_DIR="$(dirname "$TARGET_DIR")"
if [ -d "$PARENT_DIR/.git" ]; then
    echo " -> 检测到父级 Git 仓库 ($PARENT_DIR)，执行强制同步..."
    cd "$PARENT_DIR"
    git fetch --all --prune
    git reset --hard origin/main
    cd "$TARGET_DIR"
elif [ -d "$TARGET_DIR/.git" ]; then
    echo " -> 检测到当前 Git 仓库，执行强制同步..."
    git fetch --all --prune
    git reset --hard origin/main
else
    echo " -> 当前非 Git 仓库，从官方源下载最新包替换源码..."
    TMP_ARCHIVE="/tmp/vrcdog-latest.tar.gz"
    TMP_DIR="/tmp/vrcdog-extract-$$"
    mkdir -p "$TMP_DIR"

    # 优先尝试国内高速镜像，失败自动回退官方源
    if ! curl -fsSL --connect-timeout 8 "https://ghfast.top/https://github.com/KingXiaoTaoOVO/VRCDog/archive/refs/heads/main.tar.gz" -o "$TMP_ARCHIVE"; then
        echo " -> 高速代理超时，尝试直连 GitHub..."
        curl -fsSL --connect-timeout 15 "https://github.com/KingXiaoTaoOVO/VRCDog/archive/refs/heads/main.tar.gz" -o "$TMP_ARCHIVE"
    fi

    tar -xzf "$TMP_ARCHIVE" -C "$TMP_DIR"

    # 清理并替换旧文件（严格保留 data 数据库与 .env 配置）
    rm -rf "$TARGET_DIR/src"
    cp -rf "$TMP_DIR/VRCDog-main/vrcdog-server/src" "$TARGET_DIR/"
    cp -f "$TMP_DIR/VRCDog-main/vrcdog-server/Cargo.toml" "$TARGET_DIR/"
    cp -f "$TMP_DIR/VRCDog-main/vrcdog-server/Cargo.lock" "$TARGET_DIR/"
    cp -f "$TMP_DIR/VRCDog-main/vrcdog-server/Dockerfile" "$TARGET_DIR/"
    cp -f "$TMP_DIR/VRCDog-main/vrcdog-server/docker-compose.yml" "$TARGET_DIR/"
    cp -f "$TMP_DIR/VRCDog-main/vrcdog-server/README.md" "$TARGET_DIR/"
    [ -f "$TMP_DIR/VRCDog-main/vrcdog-server/update.sh" ] && cp -f "$TMP_DIR/VRCDog-main/vrcdog-server/update.sh" "$TARGET_DIR/"

    rm -rf "$TMP_ARCHIVE" "$TMP_DIR"
fi

# 3. 确保数据目录权限合规
echo "[3/4] 校验持久化数据目录及权限..."
mkdir -p "$TARGET_DIR/data"
chown -R 10001:10001 "$TARGET_DIR/data"
chmod -R 777 "$TARGET_DIR/data"
chcon -Rt svirt_sandbox_file_t "$TARGET_DIR/data" 2>/dev/null || true

# 4. 重新构建镜像并后台启动
echo "[4/4] 重新构建镜像并启动容器..."
docker compose build --no-cache
docker compose up -d

echo "=================================================="
echo " [VRCDog-Server] 更新完成！正在等待服务启动..."
echo "=================================================="

sleep 3

# 5. 健康检查
if curl -fsSL --connect-timeout 3 http://127.0.0.1:21451/ping 2>/dev/null | grep -q "ok" || \
   curl -fsSL --connect-timeout 3 http://127.0.0.1:11451/ping 2>/dev/null | grep -q "ok"; then
    echo " -> 健康检查通过: 服务正常运行中！"
else
    echo " -> 容器已启动，最新运行日志如下："
fi

docker compose logs --tail=15
