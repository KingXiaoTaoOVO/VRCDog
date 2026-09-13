# VRCDog 发布与打包指南

本文档覆盖 VRCDog 从代码修改到发布新版本、本地打包的完整工作流。

> **最后更新**：2026-09-13（v5.5.0）
> **维护者**：KingXiaoTaoOVO

---

## 目录

1. [发布前检查](#1-发布前检查)
2. [版本号同步](#2-版本号同步)
3. [代码验证](#3-代码验证)
4. [提交与打 Tag](#4-提交与打-tag)
5. [CI 自动构建](#5-ci-自动构建)
6. [本地打包（bun）](#6-本地打包bun)
7. [发布命名规范](#7-发布命名规范)
8. [常见问题与排坑](#8-常见问题与排坑)

---

## 1. 发布前检查

确认以下文件已包含本次变更：

- `src-tauri/src/update.rs` — 自动更新模块（如果有更新流程变更）
- `src/components/LoginView.vue` — 登录页静默预检测
- `src/components/SettingsView.vue` — 设置页更新操作
- `src/components/PcLayout.vue` — 侧边栏问卷中心入口
- `src/i18n/locales/zh-CN.json` + `en-US.json` — 新增 i18n key
- `.github/workflows/release.yml` — CI 发布工作流
- `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` — 版本号

---

## 2. 版本号同步

三处文件必须同步更新，缺一不可：

| 文件 | 字段 | 示例 |
|------|------|------|
| `package.json` | `"version"` | `"5.0.9"` |
| `src-tauri/tauri.conf.json` | `"version"` | `"5.0.9"` |
| `src-tauri/Cargo.toml` | `version` | `version = "5.0.9"` |

**注意**：版本号不带 `v` 前缀，纯数字 `x.y.z` 格式。Tag 才带 `v` 前缀（如 `v5.0.9`）。

---

## 3. 代码验证

发布前必须全部通过：

```bash
# 1. Rust 编译检查
cd src-tauri && cargo check

# 2. Rust 单元测试（至少 update 模块）
cd src-tauri && cargo test --lib update::

# 3. TypeScript 类型检查（本机 pnpm 损坏时用 node 直跑）
node node_modules/vue-tsc/bin/vue-tsc.js --noEmit

# 4. 前端生产构建
node node_modules/vite/bin/vite.js build
# 或
bun run build
```

---

## 4. 提交与打 Tag

```bash
# 1. 暂存所有变更
git add -A

# 2. 提交（使用 /commit 工具确保 hook 安全）
git commit -m "v5.0.9: 修复更新黑屏cmd + 登录页静默预检测"

# 3. 推送
git push origin main

# 4. 打 tag 并推送（触发 CI）
git tag v5.0.9
git push origin v5.0.9
```

**关键约束**：
- Tag 触发 workflow 时跑的是 tag 指向的 commit 的代码（非 main HEAD）
- 修改 `release.yml` 后必须重打/移动 tag 才能生效
- 删除旧 tag：`git tag -d v5.0.x && git push origin :refs/tags/v5.0.x`

---

## 5. CI 自动构建

### 工作流配置

文件：`.github/workflows/release.yml`

- **名称**：`发布 VRCDog 新版本`
- **触发**：推送 `v*` 开头的 tag
- **运行环境**：`windows-latest`，Node 22，pnpm 11.10.0，Rust stable
- **产物仓库**：`KingXiaoTaoOVO/vrcdog-releases`（主仓库 VRCDog 是私有的，release 产物路由到公开的 releases 仓库）
- **Release 命名**：`发布 ${{ github.ref_name }} 新版本`（中文格式，用户指定）
- **自动更新签名**：`tauriUpdater: true` 生成签名 `updater.json` 作为官方插件兜底通道
- **签名密钥**：`TAURI_PRIVATE_KEY` / `TAURI_KEY_PASSWORD`（GitHub Secrets）
- **发布 token**：`VRCDOG_RELEASES_TOKEN`（有 `public_repo` 权限的 PAT）

### CI 产物

- `VRCDog_x.x.x_x64-setup.exe`（NSIS 安装包，约 145MB）
- `VRCDog_x.x.x_x64_zh-CN.msi`（MSI 安装包，约 203MB）
- `updater.json`（签名更新清单，供官方 updater 插件兜底）

### 查看 CI 状态

```bash
# 用 gh CLI
gh run list --limit 5
gh run view <run-id>

# 或在浏览器
# https://github.com/KingXiaoTaoOVO/VRCDog/actions
```

CI 构建时间约 20-25 分钟（首次更长，后续有 Rust 缓存）。

---

## 6. 本地打包（bun）

当需要本地生成安装包（不等 CI）时使用此流程。

### 前置条件

- 已安装 [bun](https://bun.sh/)
- `src-tauri/resources/python-runtime/` 完整（约 324MB，含 `vrcdog-runtime.json` + 全部包 + import 自检通过）

### 标准流程

```bash
# 1. 构建前端
bun run build

# 2. 跳过 prepare-python-runtime，直接跑 tauri build
node scripts/tauri.mjs build --config .scratch/tauri-skip-prepare.json
```

`.scratch/tauri-skip-prepare.json` 内容：
```json
{
  "build": {
    "beforeBuildCommand": ""
  }
}
```

这会覆盖 `tauri.conf.json` 中的 `beforeBuildCommand`，跳过 `run-package-script.mjs build:desktop`（该脚本会先跑 `prepare:midishow` 再跑 `prepare:python` 再跑 `build`，其中 `prepare:python` 的 pip install 在本机经常龟速卡死）。

### 为什么跳过 prepare-python-runtime？

`prepare-python-runtime.mjs` 会从 `files.pythonhosted.org`（Fastly CDN）下载 `faster-whisper` 等 Python 包。本机网络环境下经常 55 分钟无进展。当 `src-tauri/resources/python-runtime/` 已完整时，重新下载是浪费。

### 产物路径

```
.cargo-target/release/bundle/nsis/VRCDog_5.0.9_x64-setup.exe    # ~145MB
.cargo-target/release/bundle/msi/VRCDog_5.0.9_x64_en-US.msi     # ~203MB
```

本地 Rust release 编译约 5 分钟，NSIS + MSI 打包约 12 分钟。

### bun 集成原理

`scripts/run-package-script.mjs` 通过检测 `npm_config_user_agent` 判断当前包管理器。若检测到 bun，会设置 `npm_config_user_agent="bun/1.3.14"` 后用 node 调用 `scripts/tauri.mjs`，整条链路即可走 bun。

---

## 7. 发布命名规范

用户指定所有 release 按以下中文格式命名：

| 配置项 | 值 |
|--------|-----|
| workflow name | `发布 VRCDog 新版本` |
| releaseName | `发布 ${{ github.ref_name }} 新版本` |
| releaseBody | `## VRCDog 更新发布\n\n有关此版本的详细更新内容，请参阅提交历史。\n\n如果使用安装包，请下载 VRCDog_x.x.x_x64_zh-CN.msi。\n如果有疑问，请加入我们的交流群。` |

**不要改成英文格式**。用户明确要求中文命名。

---

## 8. 服务端环境变量（v5.6.0 起）

| 变量 | 默认 | 说明 |
|------|------|------|
| `VRCDOG_SERVER_PASSWORD_BCRYPT` | 未设置 | **必填**（否则管理员接口禁用）。bcrypt 哈希，可用 `htpasswd -nbB admin <密码>` 生成 |
| `VRCDOG_REGISTER_SKIP_VERIFY` | 未设置 | 设为 `1` 可关闭注册的 VRChat 身份核验。**会重新暴露冒名风险**，仅限可信内网 |
| `VRCDOG_ALLOWED_EXTERNAL_HOSTS` | 未设置 | 逗号分隔的白名单。设置后，客户端 `allow_external_host` 请求只能访问清单内主机（含子域） |
| `VRCDOG_SERVER_TLS_CERT` / `VRCDOG_SERVER_TLS_KEY` | 未设置 | 独立服务端启用 HTTPS / wss。两者都设置才生效，见 `main.rs` 的 TLS 分支 |

> **v5.6.0 破坏性变更**：`/api/client/register` 现在会用客户端的 VRChat 会话 Cookie
> 向 `api.vrchat.cloud` 反查真实 user id，不一致即拒绝。因此**服务端必须能出网访问
> VRChat 官方 API**；无法出网的部署请显式设置 `VRCDOG_REGISTER_SKIP_VERIFY=1`。

---

## 9. 常见问题与排坑

### prepare-python-runtime pip install 卡死

**症状**：`prepare-python-runtime.mjs` 的 pip install 从 `files.pythonhosted.org` 下载包时龟速，55 分钟无进展。

**原因**：Fastly CDN 在某些网络环境下极慢。

**解决**：跳过该步骤（见 [本地打包](#6-本地打包bun)），前提是 python-runtime 已完整。

**根治建议**（未做）：`prepare-python-runtime.mjs` 应在 runtime 已完整（`vrcdog-runtime.json` 存在 + import 自检通过）时短路跳过，并给 pip 加国内镜像 `--index-url https://pypi.tuna.tsinghua.edu.cn/simple` 和超时参数。

### get-pip.py SHA-256 过期

**症状**：CI 在 `beforeBuildCommand` 阶段报 `SHA-256 mismatch for get-pip.py`。

**原因**：`bootstrap.pypa.io/get-pip.py` 是滚动文件，PyPA 每次 pip 发版都重写，pin SHA-256 必然过期。

**解决**（已修复，commit 0651757）：不再下载 get-pip.py，改从 `files.pythonhosted.org` 下载 pinned pip wheel（PyPI artifact URL 一经发布永不变），用 bsdtar 解压。

**教训**：任何"滚动更新 URL"（bootstrap.pypa.io, latest/, /download/）都不能 pin hash。要 pin 就用内容寻址/不可变 artifact。

### pnpm 损坏

**症状**：`pnpm` 命令报 `Cannot find module pnpm.mjs`。

**解决**：用 node 直跑 vue-tsc 和 vite：
```bash
node node_modules/vue-tsc/bin/vue-tsc.js --noEmit
node node_modules/vite/bin/vite.js build
```

或改用 bun。

### OS error 32 无法启动安装程序

**症状**：应用内点击更新后报 `os error 32 (ERROR_SHARING_VIOLATION)`。

**原因**：VRCDog.exe 还在运行时直接 `CreateProcess` 启动安装包，Windows Defender 抓取文件做实时扫描 + 进程自身 image mapping 残留句柄。

**解决**（v5.0.8 引入，v5.0.9 修正）：引导脚本 `bootstrapper.cmd` + schtasks 派发。详见 `src-tauri/src/update.rs` 模块文档注释。

### 更新后黑屏 cmd 窗口

**症状**：v5.0.8 更新时弹出黑屏 cmd 窗口（标题 `findstr /C:"688"`），卡住。

**原因**：v5.0.8 用 `cmd.exe /C` + `DETACHED_PROCESS | CREATE_NO_WINDOW` 派发引导脚本。正常 conhost 上 OK，但 Windows Terminal 会劫持 cmd.exe 子进程接管到 tab 里，`@echo off` 失效且 wait-loop 卡死。

**解决**（v5.0.9）：改用 `schtasks /Create /SC ONCE /TN VRCDog_Update_<pid> /TR "..." /ST <now+2s> /F` → `schtasks /Run` → `schtasks /Delete /F`。Task Scheduler 宿主是 `svchost.exe -k netsvcs`，进程不绑定任何 console session，永远不会被 Windows Terminal 接管。

### Git Credential Helper 弹窗死循环

**症状**：每次 git 操作都弹出 PortableGit 的 Credential Helper Selector UI。

**解决**：
```bash
git config --system --unset credential.helper
git config --global --add credential.helper wincred
```

### release.yml 修改后不生效

**原因**：Tag 触发 workflow 时跑的是 tag 指向的 commit 的代码，不是 main HEAD。

**解决**：修改 release.yml 后删除旧 tag，在最新 commit 重打：
```bash
git tag -d v5.0.x
git push origin :refs/tags/v5.0.x
git tag v5.0.x
git push origin v5.0.x
```

---

## 附录：版本变更历史

| 版本 | 日期 | 变更摘要 |
|------|------|----------|
| v5.6.0 | 2026-09-13 | 注册接口 VRChat 身份核验（L2）；管理员口令爆破锁定（R4）；远程协助默认 wss + 监听收敛到回环（R5）；外部请求 DNS rebinding 防护与可选白名单（R9）；敏感字符串改用 DPAPI 加密存储（S4）；midishow 请求超时补齐（B6）；好友列表增量更新（P1）；OSC 套接字复用（P6）；开启 updater 产物生成 |
| v5.5.0 | 2026-09-13 | 客户端令牌鉴权 + 服务端 HTTPS 支持；移除硬编码默认管理员密码（改为环境变量，未配置则拒绝）；更新强制 SHA-256 校验 + GitHub 官方域名白名单；TTS / VRChat 启动参数命令注入修复；远程协助加密隧道 nonce 重放防护；启动自动登录；日志敏感字段脱敏；VRPiano OSC 地址对齐 VRChat_MIDI_Player |
| v5.0.9 | 2026-08-19 | 修复更新黑屏 cmd 窗口（改用 schtasks 派发）；登录页新增静默预检测更新红点提示 |
| v5.0.8 | 2026-08-19 | 修复 OS error 32 无法启动安装程序（引入 bootstrapper.cmd + DETACHED_PROCESS 派发）；release 命名改为中文格式 |
| v5.0.7 | 2026-08-19 | 问卷中心侧边栏入口上线；本地 bun 打包流程建立 |
| v5.0.6 | 2026-08-18 | 好友状态三态修复（对齐 VRCX） |
| v5.0.5 | 2026-08-18 | 自动更新直连 GitHub Releases API + 问卷点击数据空白修复 |
