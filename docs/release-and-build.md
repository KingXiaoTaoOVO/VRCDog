# VRCDog 发布与打包指南

本文档覆盖 VRCDog 从代码修改到发布新版本、本地打包的完整工作流。

> **最后更新**：2026-09-26（v5.6.6）
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
8. [服务端环境变量（v5.6.0 起）](#8-服务端环境变量v560-起)
9. [常见问题与排坑](#9-常见问题与排坑)
10. [已知待办：Bilibili 会话凭据明文存储](#10-已知待办bilibili-会话凭据明文存储)

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
| `VRCDOG_ALLOWED_EXTERNAL_HOSTS` | 未设置 | 逗号分隔的白名单。设置后，客户端 `allow_external_host` 请求只能访问清单内主机（含子域），且清单内主机会跳过 DNS rebinding 检查 —— **内网自建服务端（域名解析到私有地址）必须加到这里才能连通** |
| `VRCDOG_SERVER_TLS_CERT` / `VRCDOG_SERVER_TLS_KEY` | 未设置 | 独立服务端启用 HTTPS / wss。两者都设置才生效，见 `main.rs` 的 TLS 分支 |

> **v5.6.0 破坏性变更**：`/api/client/register` 现在会用客户端的 VRChat 会话 Cookie
> 向 `api.vrchat.cloud` 反查真实 user id，不一致即拒绝。因此**服务端必须能出网访问
> VRChat 官方 API**；无法出网的部署请显式设置 `VRCDOG_REGISTER_SKIP_VERIFY=1`。

---

## 9. 常见问题与排坑

### updater 签名密钥不可用（当前状态）

`tauri.conf.json` 的 `bundle.createUpdaterArtifacts` 目前是 `false`，因此 **CI 不会产出
`updater.json`**，`tauri.conf.json` 里那个 `/releases/latest/download/updater.json`
端点仍然是死链。应用内更新走的是直连 GitHub Releases API，不影响用户更新。

**为什么不开**：v5.6.0 尝试开启后 CI 连续报两个错——

1. `A public key has been found, but no private key`  
   → Tauri v2 读的是 `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，
   而仓库旧 Secrets 只有 v1 名字 `TAURI_PRIVATE_KEY` / `TAURI_KEY_PASSWORD`。
   已在 `release.yml` 里补齐 v2 变量名（复用同一份密钥）。
2. `incorrect updater private key password: Wrong password for that key`  
   → `TAURI_PRIVATE_KEY` 与 `TAURI_KEY_PASSWORD` 这两个 Secret **内容本身不匹配**，
   无法通过代码修复，必须人工轮换。

**修好步骤**：跑 `python scripts/rotate-updater-keys.py` 生成新密钥对 →
用新公钥覆盖 `tauri.conf.json` 的 `plugins.updater.pubkey` →
把新私钥与口令写入仓库 Secrets（`TAURI_PRIVATE_KEY` + `TAURI_KEY_PASSWORD`，
v2 变量由 release.yml 自动复用）→ 把 `createUpdaterArtifacts` 改回 `true`。

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

## 10. 已知待办：Bilibili 会话凭据明文存储

**状态**：已确认，计划随下个功能版一起修复（不单独发补丁版）。
**发现时间**：2026-09-13，v5.6.2 发布后的安全复查。属于 S4 同类问题，且为历史遗留、非 v5.6.x 引入。

### 现状

Bilibili 的三个会话凭据走的是通用设置通道 `DbApi.saveSetting`：

| 键 | 含义 |
|------|------|
| `bili_sessdata` | SESSDATA，完整会话令牌，等同密码 |
| `bili_jct` | CSRF token |
| `bili_buvid3` | 设备标识 |

- **桌面端**：`db_save_setting` 明文写入 SQLite 的 `app_settings` 表（`src-tauri/src/db.rs:630`）
- **Web 端**：明文写入 `localStorage` 的 `vrcdog_setting_<key>`（`src/api/index.ts:1582`）

### 严重度：中等

- 桌面端需要本机文件访问（恶意软件、他人接触电脑）才能读到，不是远程可利用
- Web 端需要 XSS 才能读到
- 日志侧已经安全：`src/api/index.ts:29` 的 `SENSITIVE_ARG_KEYS` 已覆盖
  `sessdata` / `bili_jct` / `buvid3`，不会泄漏到日志

### 修复方案（下个功能版执行）

参照 v5.6.1 处理 Translator API Key 的方式：

1. 在设置通道里把这三个键识别为敏感键
2. 桌面端改走 `sys_store_secure_string` / `sys_load_secure_string`（DPAPI / ChaCha20 加密）
3. Web 端改走 `sessionStorage`（标签页级，关闭即清除）
4. 读取时先走新路径，取不到再回退旧值并迁移，迁移完成后清除旧值

读写点约 11 处，集中在 `src/components/BilidownView.vue` 与 `src/components/DanmakuView.vue`。
改动会触及 Bilibili 登录流程，修复后必须实测两条路径：
「登录 → 重启应用 → 凭据仍在」与「退出登录 → 凭据已清除」。

## 附录：版本变更历史

| 版本 | 日期 | 变更摘要 |
|------|------|----------|
| v5.6.6 | 2026-09-26 | 重写「绘画工作台」的图像转线稿管线，改为**按图片类型自动适配**：局部自适应阈值判定真线稿（细笔画直方图 + 距离变换厚度检验）走阈值路径，照片/立绘走 bilateral 保边滤波 → 多尺度 XDoG（阈值由 DoG 响应分位数自动反推，尺度 0.8/1.3/2.2 对应目标墨量 4%/5.5%/8%）∪ Canny 细节层 → 亮背景前景轮廓补全；沿笔画滑动平均（窗口 7）+ RDP 1.6 消除骨架抖动，共线端点按切线方向合并。修复两个真实缺陷：8 邻域闭运算把 1px 边缘遮罩膨胀 43% 并沿对角粘连相邻线（改用 4 邻域 plus 核，降到 8%）；despeckle 阈值 14px 与原型 6px 不一致导致细节被吞掉一半。AI 模式原先对 1px 边缘做开运算、恒返回 0 笔画，现改为 XDoG 强去噪路径。新增 5 个单元测试与逐阶段遮罩诊断；src-tauri 79 项、前端 88 项测试全绿 |
| v5.6.5 | 2026-09-25 | 修复 2FA 登录后自动跳出登录并清除会话；UI 全量 Emoji 规范化 |
| v5.6.4 | 2026-09-22 | 深度对标 shgeum/VRCLT 重构同传翻译交流系统：新增 VRChat / Discord / Custom 三大场景模式、Avatar OSC 说话状态同步 (/avatar/parameters/VRCT_IsTalking)、原生麦克风零延迟直通虚拟声卡 (Raw Mic Passthrough)、通义千问 Qwen3.5 LiveTranslate 国内外双节点与 DashScope Workspace ID 隔离；规范全界面 UI 图标体系（移除 emoji）；保持全套自动化测试 100% 通过 |
| v5.6.3 | 2026-09-20 | 修复自动弹琴在游戏中失灵（补充硬件扫描码与前台焦点激活机制、优化按键保持时间）；纠正悬浮窗点击交互（单击软件内试听、双击游戏中播放）；多引擎输出（键盘/MIDI/OSC）与状态无缝联动；全局快捷键全系统响应与持久化生效 |
| v5.6.2 | 2026-09-13 | 修复 v5.6.0 引入的回归：R9 的 DNS rebinding 检查会误杀「域名解析到私有地址」的内网自建服务端；现在命中 VRCDOG_ALLOWED_EXTERNAL_HOSTS 白名单的主机直接放行、跳过 DNS 检查 |
| v5.6.1 | 2026-09-13 | 修复 API Key 仍明文落 localStorage：v5.6.0 的 `useStorage` 是无条件写 localStorage，Tauri 模式下 DPAPI 加密被绕过；现在 Tauri 只写加密存储，Web 退到 sessionStorage，并迁移清除旧明文 |
| v5.6.0 | 2026-09-13 | 注册接口 VRChat 身份核验（L2）；管理员口令爆破锁定（R4）；远程协助默认 wss + 监听收敛到回环（R5）；外部请求 DNS rebinding 防护与可选白名单（R9）；敏感字符串改用 DPAPI 加密存储（S4）；midishow 请求超时补齐（B6）；好友列表增量更新（P1）；OSC 套接字复用（P6）。**updater 产物仍关闭**：仓库 Secrets 里的签名私钥与口令不匹配，需先轮换密钥 |
| v5.5.0 | 2026-09-13 | 客户端令牌鉴权 + 服务端 HTTPS 支持；移除硬编码默认管理员密码（改为环境变量，未配置则拒绝）；更新强制 SHA-256 校验 + GitHub 官方域名白名单；TTS / VRChat 启动参数命令注入修复；远程协助加密隧道 nonce 重放防护；启动自动登录；日志敏感字段脱敏；VRPiano OSC 地址对齐 VRChat_MIDI_Player |
| v5.0.9 | 2026-08-19 | 修复更新黑屏 cmd 窗口（改用 schtasks 派发）；登录页新增静默预检测更新红点提示 |
| v5.0.8 | 2026-08-19 | 修复 OS error 32 无法启动安装程序（引入 bootstrapper.cmd + DETACHED_PROCESS 派发）；release 命名改为中文格式 |
| v5.0.7 | 2026-08-19 | 问卷中心侧边栏入口上线；本地 bun 打包流程建立 |
| v5.0.6 | 2026-08-18 | 好友状态三态修复（对齐 VRCX） |
| v5.0.5 | 2026-08-18 | 自动更新直连 GitHub Releases API + 问卷点击数据空白修复 |
