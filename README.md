<div align="center">

# 🐶 VrcDog — VRChat 全能社交管家

[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://v2.tauri.app/)
[![Vue](https://img.shields.io/badge/Vue-3.x-green?logo=vuedotjs)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/Rust-Backend-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**VrcDog** 是一款基于 **Tauri 2 + Vue 3 + Rust** 构建的跨平台 VRChat 伴侣应用。  
它融合了 **VRCX**（好友管理/数据分析）、**VRCT**（语音识别翻译）、**OVR Overlay Translator**（VR 空间 OCR 翻译）三大参考项目的核心功能于一体，为 VRChat 玩家提供从桌面管理到 VR 沉浸式翻译的一站式体验。

</div>

---

## ✨ 核心功能清单

### 📊 社交管理与数据面板（源自 VRCX）

| 功能模块 | 说明 | 对应组件 |
|---------|------|---------|
| **总览大屏** | 在线好友数、活跃实例、热力图、服务器状态一目了然 | `DashboardView` |
| **社交大厅** | 好友列表分在线/离线展示，支持加好友/删好友/邀请入房 | `FriendsListView` |
| **好友日志** | 记录好友上下线、位置变更、状态切换等历史事件 | `FriendLogView` |
| **好友位置** | 按世界实例分组实时展示在线好友的位置 | `FriendLocationsView` |
| **动态星球** | 实时活动信息流（上下线、加入/离开世界等） | `FeedView` |
| **数据统计** | 图表化展示好友在线率、最活跃时段、事件趋势 | `ChartsView` |
| **房间玩家** | 查看当前游戏实例中所有玩家列表 | `PlayerListView` |
| **全局搜索** | 搜索用户、世界、模型，支持直接粘贴 ID 或 URL | `SearchView` |
| **通知中心** | 接收/回复邀请、好友请求等通知 | `NotificationsView` |
| **我的模型** | 管理和切换自有 Avatar 模型 | `MyAvatarsView` |
| **收藏夹** | 收藏世界和模型（本地 + 云端双向同步去重） | `FavoritesView` |
| **群组管理** | 查看和管理所属 VRChat 群组 | `GroupsView` |
| **安全护盾** | 管理拉黑/静音/隐藏模型用户名单 | `ModerationView` |
| **活跃热力图** | 按星期 × 小时可视化用户在线规律 | `HeatmapView` |
| **游戏日志** | 解析 VRChat 本地日志文件，展示加入/离开世界记录 | `GameLogView` |
| **回忆图库** | 管理 VRChat 游戏截图 | `GalleryView` |
| **备忘录** | 为好友添加个人备注（如何认识、Discord 名等） | `NotesView` |
| **状态预设** | 保存常用 status + description 组合，一键切换 | `StatusPresetsView` |
| **数据导出** | JSON 格式导出全部本地数据 | `ExportView` |
| **环境管家** | 检测/安装 Unity Hub、Unity 2022、VCC/ALCOM | `EnvView` |
| **高级设置** | 语言、代理、Discord RPC、缓存清理、开发者调试 | `SettingsView` |

### 🌐 语音识别与翻译系统（源自 VRCT）

| 功能 | 说明 | 对应组件 |
|-----|------|---------|
| **麦克风语音识别** | 支持云端原生引擎和本地 Whisper 双模式 | `TranslatorView` |
| **系统音频内录** | 监听游戏内他人语音进行实时翻译 | `TranslatorView` |
| **多语言翻译** | 支持 zh/en/ja/ko/fr/de 等多语种互译 | `TranslatorView` |
| **VRChat OSC 发送** | 翻译结果自动发送到 VRChat Chatbox | `TranslatorView` |
| **TTS 语音播报** | 支持系统 TTS 和 GPT-SoVITS 两种引擎 | `TranslatorView` |
| **翻译 Overlay 窗口** | 独立悬浮翻译窗口（Tauri WebviewWindow） | `OverlayView` |

### 🥽 VR 空间 OCR 翻译（源自 OVR Overlay Translator）

| 功能 | 说明 | 对应组件 |
|-----|------|---------|
| **OCR 配置** | 中英日/繁中/韩文/拉丁多语言模型选择 | `OvrTranslatorView` |
| **速度策略** | 极速/标准/精准三档模式 | `OvrTranslatorView` |
| **图像预处理** | 对比度增强、锐化等预处理开关 | `OvrTranslatorView` |
| **翻译服务** | 内置云服务或自定义 API（腾讯/DeepSeek/OpenAI） | `OvrTranslatorView` |
| **叠加层外观** | 自定义文本/背景颜色、透明度、状态指示灯色 | `OvrTranslatorView` |
| **跟随模式** | 世界锁定 / 头部锁定两种叠加层定位方式 | `OvrTranslatorView` |
| **手腕显示** | 翻译结果缩小固定在手腕（手表模式） | `OvrTranslatorView` |
| **硬件加速** | CPU 多线程 / GPU 加速（实验性）配置 | `OvrTranslatorView` |
| **自动启动** | 注册 SteamVR 启动清单 | `OvrTranslatorView` |

### 🔧 系统级工具

| 功能 | 说明 | 对应组件 |
|-----|------|---------|
| **工具箱** | VRChat 启动器、OSC 参数调试、缓存清理 | `ToolsView` |
| **调试控制台** | 全局 API 调用日志实时查看 | `DebugConsole` |
| **Discord RPC** | 在 Discord 展示当前 VRChat 活动详情 | `SettingsView` |
| **SteamVR 检测** | 登录后选择 PC / VR 模式，VR 模式自动检测 SteamVR | `App.vue` |

---

## 🏗️ 技术架构

```
VrcDog/
├── src/                          # Vue 3 前端
│   ├── api/                      # API 层
│   │   ├── index.ts              # VrcApi / DbApi / SysApi 统一接口
│   │   ├── websocket.ts          # VRChat Pipeline WebSocket 实时推送
│   │   └── gamelogWatcher.ts     # 游戏日志文件监听器
│   ├── components/               # 31 个 Vue SFC 组件
│   ├── i18n/locales/             # 国际化（中/英/日 三语言完整适配）
│   │   ├── zh-CN.json
│   │   ├── en-US.json
│   │   └── ja-JP.json
│   ├── types/vrc.ts              # TypeScript 类型定义
│   └── App.vue                   # 主应用（登录/模式选择/路由）
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── lib.rs                # Tauri 指令注册与插件配置
│       ├── db.rs                 # SQLite 数据库（好友/日志/收藏/设置/通知）
│       ├── vrc_api.rs            # VRChat API HTTP 代理（绕 CORS）
│       ├── sys.rs                # 系统指令（OSC/SteamVR检测/文件操作）
│       ├── hardware.rs           # 硬件检测（Unity/Hub/VCC/ALCOM）
│       ├── gallery.rs            # VRChat 截图读取
│       └── gamelog.rs            # 游戏日志解析
├── src-python/                   # Python 音频捕获模块
│   └── vrcdog_audio.py           # 系统音频/麦克风捕获与 Whisper 识别
└── OVR/ VRCT/ VRCX/             # 参考项目源码（仅供设计参考）
```

### 核心技术栈

| 层级 | 技术 | 说明 |
|-----|------|------|
| **前端框架** | Vue 3 + Composition API | SFC 组件化，Reactive 状态管理 |
| **桌面壳** | Tauri 2 | 轻量跨平台，安全的 IPC 通信 |
| **后端** | Rust | 高性能 HTTP 代理、SQLite 操作、系统调用 |
| **样式** | TailwindCSS | 暖色调主题（Amber/Orange 色系） |
| **国际化** | vue-i18n | 中文/英文/日文三语言完整覆盖 |
| **图标** | Lucide Vue Next | 统一矢量图标库 |
| **数据库** | SQLite (rusqlite) | 本地离线缓存，无需服务器 |
| **实时通信** | WebSocket | VRChat Pipeline 事件流订阅 |
| **音频处理** | Python (可选) | Whisper 本地语音识别引擎 |

---

## 🚀 快速开始

### 环境要求

- **Node.js** >= 18
- **pnpm** >= 8
- **Rust** (stable toolchain)
- **Windows 10/11** (主要支持平台)

### 安装与运行

```bash
# 1. 克隆仓库
git clone <repo-url> && cd VrcDog

# 2. 安装前端依赖
pnpm install

# 3. 启动开发模式（前端 + Rust 后端同步编译）
pnpm run tauri dev

# 4. 构建生产包
pnpm run tauri build
```

### VR 模式额外要求

- 安装并启动 **SteamVR**
- 登录后选择 **VR Overlay** 模式
- 应用会自动检测 `vrserver.exe` / `vrmonitor.exe` 进程

---

## 🌍 国际化支持

VrcDog 完整支持三种语言，覆盖全部 31 个组件的所有界面文本：

| 语言 | 语言文件 | 字符串数 |
|------|---------|---------|
| 🇨🇳 简体中文 | `zh-CN.json` | 600+ |
| 🇺🇸 English | `en-US.json` | 600+ |
| 🇯🇵 日本語 | `ja-JP.json` | 600+ |

语言可在「高级设置 → 常规 → 界面语言」中切换。

---

## 📁 项目文件说明

| 文件/目录 | 用途 |
|----------|------|
| `src/` | Vue 3 前端源码 |
| `src-tauri/` | Rust 后端源码 |
| `src-python/` | Python 音频处理模块（可选） |
| `OVR/` | OVR Overlay Translator 参考资料 |
| `VRCT/` | VRCT 语音翻译参考源码 |
| `VRCX/` | VRCX 伴侣应用参考源码 |
| `public/` | 静态资源 |
| `.vscode/` | VS Code 编辑器配置 |

---

## 📋 功能集成对照表

### vs VRCX

| VRCX 功能 | VrcDog 状态 | 备注 |
|-----------|------------|------|
| 好友列表管理 | ✅ 已集成 | 含在线/离线分组、加好友/删好友 |
| 好友活动监控 | ✅ 已集成 | WebSocket 实时推送 + Feed |
| 好友位置跟踪 | ✅ 已集成 | 按实例分组展示 |
| 自定义 Dashboard | ✅ 已集成 | 总览大屏 + 热力图 |
| 全局搜索 | ✅ 已集成 | 用户/世界/模型搜索 |
| 活跃热力图 | ✅ 已集成 | 星期 × 小时矩阵 |
| 截图图库 | ✅ 已集成 | 自动扫描 VRChat 截图目录 |
| 通知管理 | ✅ 已集成 | 邀请/好友请求 |
| 状态预设 | ✅ 已集成 | 保存和快速切换 |
| 服务器状态 | ✅ 已集成 | 实时 API 状态检测 |
| Discord RPC | ✅ 已集成 | 可配置展示文本 |
| 数据导出 | ✅ 已集成 | JSON 格式全量导出 |
| VR Overlay Feed | ✅ 已集成 | Overlay 窗口模式 |
| 模型管理 | ✅ 已集成 | 查看和切换 Avatar |
| 群组管理 | ✅ 已集成 | 查看所属群组信息 |
| 安全管理 | ✅ 已集成 | 拉黑/静音/隐藏列表 |
| 备忘录 | ✅ 已集成 | 按好友记录笔记 |
| 好友日志 | ✅ 已集成 | 上下线/位置变更历史 |

### vs VRCT

| VRCT 功能 | VrcDog 状态 | 备注 |
|-----------|------------|------|
| 麦克风语音识别 | ✅ 已集成 | 云端 + Whisper 双引擎 |
| 系统音频内录 | ✅ 已集成 | 监听他人语音翻译 |
| 多语言翻译 | ✅ 已集成 | 多语种互译 |
| OSC Chatbox 发送 | ✅ 已集成 | 自动发送翻译到 VRC |
| TTS 语音播报 | ✅ 已集成 | System + GPT-SoVITS |

### vs OVR Overlay Translator

| OVR 功能 | VrcDog 状态 | 备注 |
|----------|------------|------|
| OCR 语言模型选择 | ✅ 已集成 | 6 种语言模型 |
| 速度/精度策略 | ✅ 已集成 | 三档切换 |
| 图像预处理增强 | ✅ 已集成 | 对比度/锐化 |
| 翻译服务配置 | ✅ 已集成 | 内置 + 自定义 |
| 叠加层外观定制 | ✅ 已集成 | 颜色/透明度/跟随模式 |
| 手腕常驻显示 | ✅ 已集成 | 手表模式配置 |
| 硬件加速 | ✅ 已集成 | CPU/GPU 加速开关 |
| SteamVR 自动启动 | ✅ 已集成 | 启动清单注册 |
| Dashboard 设置面板 | ✅ 已集成 | 5 个子标签页完整实现 |

---

## ⚠️ 声明

VrcDog 不隶属于 VRChat Inc.，也不代表 VRChat 或任何官方相关方的观点或立场。VRChat 及所有相关标识为 VRChat Inc. 的商标或注册商标。

本项目通过 VRChat 公开 API 提供功能，不修改游戏本体，不包含任何作弊或修改行为。

---

<div align="center">

**Made with 🐾 by the VrcDog Team**

</div>
