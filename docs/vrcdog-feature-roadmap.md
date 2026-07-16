# VrcDog 一比一复刻完整开发文档

参考源：`禁止移除！/VrcDog/`（MIT 协议，pypy-vrc / Natsumi-sama / Map1en 等贡献者）
我们的目标：在 Tauri + Vue 3 架构下一比一复刻 VrcDog 的全部界面与核心功能，使用我们自己的主题、i18n、Tauri 后端命令和 SQLite 数据层。

---

## 目录

1. [架构总览](#1-架构总览)
2. [VrcDog 完整模块清单](#2-vrcdog-完整模块清单)
3. [API 层（VRChat 接口）](#3-api-层vrchat-接口)
4. [Stores 层（状态管理）](#4-stores-层状态管理)
5. [Coordinators 层（业务逻辑）](#5-coordinators-层业务逻辑)
6. [Services 层（基础设施）](#6-services-层基础设施)
7. [Composables 层（组合式工具）](#7-composables-层组合式工具)
8. [Shared 层（常量与工具函数）](#8-shared-层常量与工具函数)
9. [UI 组件库](#9-ui-组件库)
10. [对话框系统（Dialogs）](#10-对话框系统dialogs)
11. [视图（Views/Pages）](#11-视图viewspages)
12. [VR 模式](#12-vr-模式)
13. [Workers（后台计算）](#13-workers后台计算)
14. [国际化](#14-国际化)
15. [后端 IPC（Tauri 命令映射）](#15-后端-ipctauri-命令映射)
16. [复刻路线图（按优先级）](#16-复刻路线图按优先级)
17. [当前进度](#17-当前进度)

---

## 1. 架构总览

VrcDog 是 Vue 3 + Pinia + Vue Router + Vue I18n + ECharts + Reka UI（shadcn 风格）的桌面应用，原生层用 .NET（Windows）或 Electron（跨平台）。我们用 Tauri 2 + Rust 替代原生层。

```
┌─────────────────────────────────────────────────────────┐
│                    Vue 3 前端                            │
├─────────────────────────────────────────────────────────┤
│  views (页面)                                            │
│    ↓ 调用                                                │
│  coordinators (业务编排)                                 │
│    ↓ 读写                                                │
│  stores (Pinia 状态)                                     │
│    ↓ 调用                                                │
│  api (VRChat REST API 封装)  +  services (基础设施)      │
│    ↓ 通过                                                │
│  webapi.js (HTTP 客户端)  +  websocket.js  +  database/  │
└─────────────────────────────────────────────────────────┘
            ↕                              ↕
   VRChat API (api.vrchat.cloud)    Native (.NET / Tauri Rust)
                                          ↕
                                      SQLite
```

### 数据流约定（重要）

- **api/** — 纯函数，只发 HTTP 请求，不存状态。每个模块对应 VRChat API 的一个资源（auth / user / world / avatar / favorite / notification / group / instance ...）
- **stores/** — Pinia store，存内存状态 + reactive refs。**不能**直接发 API 请求，必须通过 coordinator 包一层
- **coordinators/** — 业务编排，把 api 调用 + store 写入 + 副作用（toast、日志、cache）拼起来。所有"业务动作"都在这里
- **services/** — 单例基础设施：HTTP 客户端、WebSocket、数据库、安全（加密）、配置存储
- **components/dialogs/** — 全局浮层对话框，由 modalStore 管理生命周期
- **components/ui/** — Reka UI 风格的 shadcn 组件库，60+ 个原子组件
- **views/** — 路由对应的页面级组件
- **composables/** — Vue 3 组合式函数（useToolActions / useUserDisplay / useImageCropper 等）
- **shared/** — 纯函数工具和常量

---

## 2. VrcDog 完整模块清单

下表是 VrcDog 全部源码模块（src/ 下的目录）的职责。

| 顶级目录 | 文件数（约） | 职责 |
|---|---|---|
| `api/` | 22 | VRChat REST API 封装。每个文件对应一个资源 |
| `components/` | 100+ | UI 组件（按对话框、UI 库、布局分组） |
| `components/dialogs/` | 60+ | 全部全局对话框（UserDialog / WorldDialog / AvatarDialog / GroupDialog / InviteDialog / ...） |
| `components/ui/` | 60+ | shadcn 风格原子组件库（Button / Dialog / DropdownMenu / Tabs / DataTable / ...） |
| `components/nav-menu/` | - | 主导航菜单组件 |
| `components/onboarding/` | - | 初次使用引导 |
| `composables/` | 8 | Vue 3 组合式函数（useToolActions / useUserDisplay / useImageCropper / useOptionKeySelect / ...） |
| `coordinators/` | 24 | 业务逻辑编排（authCoordinator / friendSyncCoordinator / userCoordinator / instanceCoordinator / ...） |
| `ipc-electron/` | 1 | Electron 跨进程通信（Windows .NET 时不使用） |
| `lib/` | 3 | 通用工具（modalPortalLayers / utils / table 库） |
| `localization/` | 14 | i18n 翻译文件（cs / en / es / fr / hu / ja / ko / pl / pt / ru / th / vi / zh-CN / zh-TW） |
| `plugins/` | 10 | Vue 插件初始化（i18n / router / dayjs / sentry / ui / noty / ...） |
| `queries/` | 5 | TanStack Query 实体缓存（client / entityCache / keys / policies / useEntityQueries） |
| `services/` | 11 + database/ | 基础设施（webapi / websocket / sqlite / security / appConfig / config / gameLog / watchState / request / jsonStorage / confusables） |
| `shared/constants/` | 22 | 全部常量（access / api / dashboard / discord / emoji / feedFilters / fonts / group / instance / language / link / moderation / settings / tags / themes / tools / ui / user / world / ...） |
| `shared/utils/` | 35+ | 纯函数工具（_utils / activityEngine / appActions / avatar / chart / common / compare / csv / discordPresence / friend / gallery / gameLog / group / imageUpload / instance / invite / location / locationParser / notificationCategory / notificationMessage / overlapCalculator / platformUtils / quickSearchUtils / resolveRef / retry / setting / throttle / urlUtils / user / world ... 加上各种 transforms） |
| `stores/` | 30+ | Pinia stores（见第 4 节） |
| `stores/settings/` | 6 | 设置子 store（advanced / appearance / discordPresence / general / notifications / wristOverlay） |
| `stores/gameLog/` | - | 游戏日志相关 store 拆分 |
| `stores/notification/` | - | 通知相关 store 拆分 |
| `styles/` | 7 | 全局样式（globals / fonts / flags / animated-emoji / status-icon / noty / themes/） |
| `types/` | 4 + api/ | TypeScript 类型定义 |
| `views/` | 18 个页面 | 路由级页面（见第 11 节） |
| `vr/` | - | VR 叠加层独立 entry point |
| `workers/` | 2 | Web Worker（activityWorker / activityWorkerRunner） |

入口文件：
- `App.vue` + `app.js` — 主程序入口
- `vr.html` + `vr/Vr.vue` + `vr/vr.js` — VR 叠加层独立入口

---

## 3. API 层（VRChat 接口）

22 个文件，每个对应 VRChat REST API 的一个资源。所有请求最终走 `services/webapi.js`，自动处理 cookie、2FA、重试、限流、错误事件。

| 文件 | 资源 | 主要端点 |
|---|---|---|
| `auth.js` | 认证 | `POST /auth/twofactorauth/{method}/verify` / `GET /auth/user` / `PUT /logout` / 2FA 流程 |
| `user.js` | 用户 | `GET /users/{id}` / `GET /users` / `PUT /users/{id}` / `GET /auth/user/notifications` 相关 |
| `friend.js` | 好友 | `GET /auth/user/friends` / `POST /user/{id}/friendRequest` / `DELETE /user/{id}/friendRequest` / `GET /user/{id}/friendStatus` |
| `world.js` | 世界 | `GET /worlds/{id}` / `GET /worlds` / `PUT /worlds/{id}` / `DELETE /worlds/{id}` |
| `avatar.js` | 头像 | `GET /avatars/{id}` / `GET /avatars` / `PUT /users/{id}/avatar` / `PUT /avatars/{id}/select` |
| `instance.js` | 实例 | `GET /instances/{location}` / `POST /instances` |
| `group.js` | 群组 | `GET /groups/{id}` / `GET /users/{id}/groups` / `POST /groups/{id}/posts` / 大量子端点 |
| `notification.js` | 通知 | `GET /auth/user/notifications` / `PUT /auth/user/notifications/{id}/see` |
| `playerModeration.js` | 玩家管理 | `GET /auth/user/playermoderations` / `POST /auth/user/playermoderations` / `PUT /auth/user/unplayermoderate` |
| `avatarModeration.js` | 头像管理 | 头像屏蔽相关 |
| `favorite.js` | 收藏 | `GET /favorite/groups` / `POST /favorites` / `DELETE /favorites/{id}` |
| `inviteMessages.js` | 邀请消息模板 | `GET /message/{id}/{type}` / `PUT /message/{id}/{type}/{slot}` |
| `image.js` | 图片上传 | `POST /file/{id}/{version}/file/start` 多步骤上传协议 |
| `vrcPlusImage.js` | VRC+ 图库 | `GET /file` / `POST /file/image` |
| `vrcPlusIcon.js` | VRC+ 图标 | 头像图标专用上传 |
| `prop.js` | 道具 | `GET /props/{id}` / 道具列表 |
| `inventory.js` | 库存 | `GET /inventory` / 物品管理 |
| `misc.js` | 杂项 | 徽章更新、`GET /config`、健康检查 |
| `queryRequest.js` | 翻译请求 | 翻译 API 客户端 |
| `index.js` | 聚合导出 | export 所有 request namespace |

**与我们的映射：**
- 我们的 `src/api/` 已有 7 个对应文件（auth / user / friend / world / avatar / favorite / file / group / notification / gamelogWatcher / websocket / request）
- 缺少：avatarModeration / inviteMessages / prop / inventory / vrcPlusIcon / queryRequest / misc

---

## 4. Stores 层（状态管理）

VrcDog 有 **30+ 个 store**，全部基于 Pinia setup-style。

### 核心 store

| Store | 职责 | 关键 state |
|---|---|---|
| `auth.js` | 认证状态 | `loginForm` / `currentUser` / `isLoggedIn` / `loginParms` / `savedCredentials` |
| `user.js` | 用户对话框 + 缓存 | `userDialog` / `cachedUsers` / `currentUser` / `previousDisplayNames` |
| `world.js` | 世界对话框 + 缓存 | `worldDialog` / `cachedWorlds` |
| `avatar.js` | 头像对话框 + 缓存 | `avatarDialog` / `cachedAvatars` |
| `group.js` | 群组对话框 + 缓存 | `groupDialog` / `cachedGroups` / `currentUserGroups` / `inGameGroupOrder` |
| `instance.js` | 实例缓存 | `cachedInstances` / `playerJoiningInstance` |
| `friend.js` | 好友状态 | `friendLog` / `friendLogTable` / `localFavoriteFriends` |
| `favorite.js` | 收藏夹 | `favoriteFriendGroups` / `favoriteWorldGroups` / `favoriteAvatarGroups` |
| `notification.js`（拆分目录） | 通知中心 | `unseenNotifications` / `notificationTable` |
| `feed.js` | 动态时间轴 | `feedTable` / 按日期分组 |
| `gameLog.js`（拆分目录） | 游戏日志 | `gameLogTable` / `gameLogSessionTable` |
| `location.js` | 当前/上次位置 | `lastLocation` / `lastLocationDestination` |
| `invite.js` | 邀请状态 | `sendInviteDialog` / 收到的邀请 |
| `moderation.js` | 玩家管理 | `playerModerationTable` |
| `vrcStatus.js` | VRChat 服务器状态 | `hasIssue` / `isMajor` / `statusText` |

### Settings 子 store（6 个）

| Store | 职责 |
|---|---|
| `settings/general.js` | 常规设置（关闭到托盘 / 自启动 / 默认页 / ...） |
| `settings/appearance.js` | 外观设置（主题 / 字体 / 颜色 / 信任色覆盖 / 隐藏备注备忘） |
| `settings/notifications.js` | 通知设置（声音 / 桌面 / TTS / 过滤） |
| `settings/discordPresence.js` | Discord RPC 设置 |
| `settings/advanced.js` | 高级设置（主密码 / 翻译 API / YouTube API / 启动参数） |
| `settings/wristOverlay.js` | VR 手腕叠加设置 |

### UI / 工具 store

| Store | 职责 |
|---|---|
| `ui.js` | UI 全局状态（侧边栏宽度 / 当前路由 / shift 按下 / ...） |
| `modal.js` | 模态对话框队列管理（confirm / alert / prompt） |
| `dashboard.js` | 仪表盘 widget 配置 |
| `charts.js` | 数据统计图表数据 |
| `gallery.js` | 图库（VRC+ 上传 / 截图） |
| `tools.js` | 工具页 |
| `quickSearch.js` + `quickSearchWorker.js` | 快速搜索（Ctrl+K） |
| `search.js` + `searchIndex.js` | 全局搜索 |
| `sharedFeed.js` | 共享动态（feed + friendLog + gameLog 合并视图） |
| `vr.js` | VR 模式状态 |
| `vrcdog.js` | VrcDog 应用本身的运行状态 |
| `vrcdogUpdater.js` | 应用更新检查 |
| `photon.js` | Photon 网络包嗅探（高级功能） |
| `activity.js` | 活动数据（用户对话框活动 tab） |
| `avatarProvider.js` | 头像数据库（社区） |
| `launch.js` | 启动 VRChat 流程 |
| `updateLoop.js` | 主循环（每秒触发的同步轮询） |
| `index.js` | 聚合导出 |

---

## 5. Coordinators 层（业务逻辑）

24 个 coordinator，每个负责一类业务流程。这是 VrcDog **最重要**的架构层 —— 所有"做事情"的逻辑都在这里。

| Coordinator | 职责 |
|---|---|
| `authCoordinator.js` | 登录流程（含 2FA）、登出、保存凭证 |
| `authAutoLoginCoordinator.js` | 启动时自动登录 |
| `userSessionCoordinator.js` | 用户会话生命周期（登录后初始化、登出清理） |
| `userCoordinator.js` | 用户对话框打开、刷新、缓存 |
| `userEventCoordinator.js` | 用户事件分发（上线/下线/状态变化） |
| `friendSyncCoordinator.js` | 好友列表同步（启动时全量 + 增量） |
| `friendPresenceCoordinator.js` | 好友在线状态更新 |
| `friendRelationshipCoordinator.js` | 加好友、删好友、好友请求 |
| `worldCoordinator.js` | 世界对话框打开、刷新、缓存 |
| `avatarCoordinator.js` | 头像对话框打开、穿戴、上传 |
| `groupCoordinator.js` | 群组对话框打开、加入退出、设置可见性、排序 |
| `instanceCoordinator.js` | 实例信息加载、玩家计数、刷新 |
| `locationCoordinator.js` | 位置变化事件、传送中状态 |
| `inviteCoordinator.js` | 发送邀请、邀请请求、邀请消息模板 |
| `moderationCoordinator.js` | 屏蔽/静音/解除 |
| `favoriteCoordinator.js` | 收藏夹增删改查、分组拖拽 |
| `gameCoordinator.js` | VRChat 进程检测、启动游戏 |
| `gameLogCoordinator.js` | 游戏日志解析、聊天消息记录 |
| `dateCoordinator.js` | 日期格式化、好友建立日期、上次见到 |
| `cacheCoordinator.js` | 缓存清理、过期策略 |
| `searchIndexCoordinator.js` | 搜索索引构建（玩家/世界/头像/群组） |
| `imageUploadCoordinator.js` | 图片上传流程 |
| `memoCoordinator.js` | 备忘录增删改 |
| `vrcdogCoordinator.js` | VrcDog 自身（自启动、托盘、深度链接、URL Scheme） |

---

## 6. Services 层（基础设施）

| Service | 职责 |
|---|---|
| `webapi.js` | HTTP 客户端核心。CookieContainer、自动 retry、限流（VRChat API 限制 ~10 req/s）、2FA 自动续期 |
| `websocket.js` | VRChat Pipeline WebSocket（实时事件：好友上下线、邀请、通知） |
| `request.js` | request 工具：批量请求 (`processBulk`)、节流、并发控制 |
| `database/` | SQLite 抽象层。表结构、迁移、CRUD |
| `sqlite.js` | SQLite 操作封装 |
| `appConfig.js` | 应用配置（注册表 / .NET LocalSettings） |
| `config.js` | configRepository 抽象（getString / setString / getBool / setBool / getInt） |
| `jsonStorage.js` | 大对象 JSON 存储（如 savedCredentials） |
| `gameLog.js` | VRChat 日志文件解析（output_log_xxxx.txt） |
| `security.js` | 主密码加密/解密（用于保护 savedCredentials） |
| `confusables.js` | 字符相似度检测（用于昵称欺骗预警） |
| `watchState.js` | 全局可观察状态（isLoggedIn / userId 等，跨 coordinator 共享） |

---

## 7. Composables 层（组合式工具）

| Composable | 用途 |
|---|---|
| `useUserDisplay.js` | 用户显示工具：`userImage(user, allowOverride, size)` / `userStatusClass(user)` / 信任色 / 平台图标 |
| `useImageCropper.js` | 图片裁剪（头像、世界缩略图） |
| `useInviteChecks.js` | 检查能否邀请（实例容量、私密性） |
| `useMainLayoutResizable.js` | 主布局可调整大小（侧边栏拖拽） |
| `useOptionKeySelect.js` | 用 key 选择 + Map 配置驱动的下拉框（多处用：排序选择、过滤选择） |
| `useRecentActions.js` | 最近操作快捷列表 |
| `useToolActions.js` | 工具页操作 |
| `useToolNavPinning.js` | 工具页导航固定 |

---

## 8. Shared 层（常量与工具函数）

### Constants（22 个）

| Constants | 内容 |
|---|---|
| `accessType.js` | 实例访问类型枚举（public / friends+ / friends / invite+ / invite / group） |
| `api.js` | VRChat API 基础 URL、版本、User-Agent |
| `dashboard.js` | 仪表盘默认 widget 配置 |
| `discord.js` | Discord RPC 应用 ID、按钮配置 |
| `emoji.js` + `remixIconTags.json` | 表情、图标映射 |
| `feedFilters.js` | 动态过滤器枚举（onlinePlayers / friendActivity / ...） |
| `fonts.js` | 可选字体列表 |
| `group.js` | 群组常量（最大数量、权限位） |
| `instance.js` | 实例常量、区域代码 |
| `language.js` | **VRChat 语言代码 → famfamfam 国家代码映射**（eng→us / jpn→jp / zho→cn / ...） |
| `link.js` | 外部链接（Discord、文档、官方网站） |
| `moderation.js` | 屏蔽类型枚举 |
| `photon.js` | Photon 网络常量 |
| `settings.js` | 设置项默认值 |
| `tags.js` | 系统标签常量（admin_* / system_* / language_* / show_* ...） |
| `themes.js` | 主题色板（dark / dark-vanillaice / light / pink / blackbarnacle / nature / ...） |
| `tools.js` | 工具页常量 |
| `ui.js` | UI 常量（侧边栏宽度、对话框尺寸） |
| `user.js` | 用户排序选项、好友排序选项、状态映射 |
| `world.js` | 世界常量（容量、能力位） |

### Utils（35+ 个）

| Utils | 用途 |
|---|---|
| `_utils.js` | 通用工具（debounce / throttle / arrayMatch） |
| `activityEngine.js` | 活动 tab 数据计算（热力图桶、最常去世界） |
| `appActions.js` | 主程序级动作（重启、退出、最小化） |
| `avatar.js` + `avatarTransforms.js` | 头像数据规范化 |
| `cacheUtils.js` | LRU 缓存 |
| `chart.js` | 图表数据格式化 |
| `common.js` | `subsetOfLanguages` / `formatDateFilter` / `timeToText` / `userOnlineFor` / `userOnlineForTimestamp` / `compareByXxx` 系列比较器 / `openExternalLink` / `getFaviconUrl` / `isRealInstance` / `isFriendOnline` |
| `compare.js` | `compareByDisplayName` / `compareByLastActive` / `compareByFriendOrder` / `compareByMemberCount` / `compareByName` |
| `csv.js` | CSV 导入导出（群组成员管理） |
| `discordPresence.js` | Discord RPC 字符串拼装 |
| `entityTransforms.js` | 通用 ref 转换（detectChangedProps） |
| `fileUtils.js` | 文件工具（路径、扩展名） |
| `friend.js` | 好友计数、在线分类 |
| `gallery.js` | 图库元数据 |
| `gameLog.js` | 游戏日志格式化 |
| `group.js` + `groupTransforms.js` | 群组数据规范化 |
| `imageUpload.js` | 图片上传辅助 |
| `instance.js` + `instanceTransforms.js` | 实例数据规范化、URL 解析 |
| `invite.js` | 邀请检查 |
| `location.js` + `locationParser.js` | 位置字符串解析（`wrld_xxx:12345~public` → 结构化对象） |
| `notificationCategory.js` + `notificationMessage.js` + `notificationTransforms.js` | 通知分类、消息生成 |
| `overlapCalculator.js` | 活动重叠度计算（你和好友同时在线的时间） |
| `platformUtils.js` | 平台检测（PC / Quest / iOS） |
| `quickSearchUtils.js` | 快速搜索分词 |
| `resolveRef.js` | 实体引用解析（id → 对象，跨 cache 查找） |
| `retry.js` | 带退避重试 |
| `setting.js` | 设置读写辅助 |
| `throttle.js` | 限流 |
| `urlUtils.js` | URL 处理 |
| `user.js` + `userTransforms.js` | 用户数据规范化（含 `applyUserLanguage` / `computeUserPlatform` / `languageClass` / `getUserState` / `userColour`） |
| `world.js` + `worldTransforms.js` | 世界数据规范化 |
| `localizationHelperCLI.js` | 翻译辅助 CLI 工具（开发用） |

---

## 9. UI 组件库

VrcDog 用 Reka UI（Vue 3 版的 shadcn）构建了完整的设计系统，60+ 个原子组件。结构都是 `XxxRoot.vue / XxxItem.vue / XxxContent.vue / XxxTrigger.vue` 模式。

| 组件类别 | 组件 |
|---|---|
| 布局 | Card / Sheet / Sidebar / ScrollArea / Resizable / Separator / Skeleton |
| 输入 | Input / InputGroup / InputOTP / Textarea / Checkbox / RadioGroup / Switch / Slider / Toggle / ToggleGroup / NumberField / TagsInput / NativeSelect / Select / VirtualCombobox |
| 反馈 | Alert / AlertDialog / Dialog / Sonner（toast）/ Spinner / Progress / Tooltip / HoverCard |
| 导航 | Tabs / TabsUnderline（VrcDog 自定义下划线版本）/ Breadcrumb / Pagination |
| 菜单 | DropdownMenu / ContextMenu / Command（Cmd+K 风格搜索） / Popover |
| 数据展示 | Avatar（含 Image/Fallback）/ Badge / Item（list-style）/ Table / DataTable / Tree / Calendar / RangeCalendar |
| 表单 | Form / Field / Label / Kbd（键盘快捷键）/ Empty（空状态）|
| 其他 | Carousel / ButtonGroup |

每个组件有：`Component.vue` + `index.js`（导出 + 工具）+ 部分有 `context.js`（provide/inject）。

---

## 10. 对话框系统（Dialogs）

VrcDog 把所有"全局浮层"叫 dialog。统一通过 `MainDialogContainer.vue` 装载，通过 `modalStore` 控制生命周期。

### 顶层独立对话框（13 个）

| 对话框 | 用途 |
|---|---|
| `MainDialogContainer.vue` | 容器（路由级，载入所有子对话框） |
| `ChooseFavoriteGroupDialog.vue` | 选择收藏分组（添加好友/世界/头像到收藏） |
| `CustomNavDialog.vue` | 自定义导航菜单 |
| `DatabaseUpgradeDialog.vue` | 数据库升级进度 |
| `DialogJsonTab.vue` | 通用 JSON 树展示 tab |
| `ImageCropDialog.vue` | 图片裁剪（头像 / 世界缩略图） |
| `InviteGroupDialog.vue` | 群组邀请 |
| `LaunchDialog.vue` | 启动 VRChat（实例选择 / 启动参数） |
| `ModerateGroupDialog.vue` | 屏蔽群组 |
| `SendBoopDialog.vue` | Boop（戳一戳） |
| `SortableTreeNode.vue` | 可拖拽树节点 |
| `TableLimitsDialog.vue` | 数据表行数限制 |
| `VrcDogUpdateDialog.vue` | VrcDog 自身更新 |

### UserDialog 子对话框（10 个）

`components/dialogs/UserDialog/`

| 文件 | 功能 |
|---|---|
| `UserDialog.vue` | 主框架（Header + 7 个 tab） |
| `UserSummaryHeader.vue` | 头像/状态/名字/旗帜/徽章/平台/动作菜单 |
| `UserDialogInfoTab.vue` | Info tab（实例 + 房间玩家 + 备注 + 代表群组 + Bio + 加入次数 + 一起时长 + ID） |
| `UserDialogMutualFriendsTab.vue` | 共同好友 tab（搜索 + 排序） |
| `UserDialogGroupsTab.vue` | 群组 tab（拥有/共同/其他 + 编辑模式） |
| `UserDialogGroupCard.vue` | 群组卡片 |
| `UserDialogWorldsTab.vue` | 创建的世界 tab |
| `UserDialogFavoriteWorldsTab.vue` | 收藏的世界 tab |
| `UserDialogAvatarsTab.vue` | 创建的模型 tab |
| `UserDialogActivityTab.vue` | 活动 tab（热力图 + 重叠度 + 每日时长 + 最常去世界） |
| `BioDialog.vue` | 编辑自己的 bio + bioLinks |
| `EditNoteAndMemoDialog.vue` | 编辑用户备注和备忘 |
| `LanguageDialog.vue` | 语言多选 |
| `PronounsDialog.vue` | 编辑代词 |
| `SocialStatusDialog.vue` | 编辑社交状态 + 历史记录 |
| `SendInviteRequestDialog.vue` | 发送邀请请求 |
| `UserActionDropdown.vue` | More 动作菜单 |
| `useUserDialogCommands.js` | 用户对话框命令编排 |
| `activity/buildHeatmapOption.js` | ECharts 热力图配置 |
| `activity/DailyPlaytime.vue` | 每日游戏时长折线图 |

### WorldDialog 子对话框（5 个）

| 文件 | 功能 |
|---|---|
| `WorldDialog.vue` | 主框架（Header + Info/Instances/JSON tab） |
| `WorldDialogInfoTab.vue` | 世界基本信息 |
| `WorldDialogInstancesTab.vue` | 当前实例列表 |
| `WorldAllowedDomainsDialog.vue` | 视频播放允许域名 |
| `SetWorldTagsDialog.vue` | 设置世界标签 |
| `useWorldDialogCommands.js` + `useWorldDialogInfo.js` | 命令 + 信息编排 |

### AvatarDialog 子对话框（3 个）

| 文件 | 功能 |
|---|---|
| `AvatarDialog.vue` | 主框架 |
| `SetAvatarStylesDialog.vue` | 设置头像风格 |
| `SetAvatarTagsDialog.vue` | 设置头像标签 |
| `useAvatarDialogCommands.js` | 命令编排 |

### GroupDialog 子对话框（22 个）

包含主对话框 + 4 个 tab + 群组成员审核子系统（11 个文件）：
- 主框架 + Info/Members/Photos/Posts tab
- 成员管理：MembersTab / BansTab / InvitesTab / LogsTab + 批量操作 + 导入导出
- 群组帖子编辑、画廊选择
- 多个 composable（`useGroupBatchOperations` / `useGroupCalendarEvents` / `useGroupGalleries` / `useGroupMembers` / `useGroupModerationData` / `useGroupModerationSelection`）

### InviteDialog 子对话框（5 个）

| 文件 | 功能 |
|---|---|
| `InviteDialog.vue` | 收到的邀请列表 |
| `SendInviteDialog.vue` | 发送邀请 |
| `EditAndSendInviteDialog.vue` | 编辑后发送 |
| `SendInviteConfirmDialog.vue` | 发送确认 |

### NewInstanceDialog（创建新实例）

实例参数构建器（regions / accessType / queue 等）。

### PreviousInstancesDialog（历史实例）

列表视图 + 信息卡片 + 折线图（玩家进出频率）。

---

## 11. 视图（Views/Pages）

18 个路由级页面：

### Layout / Sidebar / Login

| 页面 | 用途 |
|---|---|
| `Layout/MainLayout.vue` | 主框架（含 sidebar、router-view、状态栏） |
| `Sidebar/Sidebar.vue` | 侧边栏（主导航 + 好友列表 + 群组）+ 3 个工具脚本 |
| `Login/Login.vue` | 登录页（含已保存账号） + `Dialog/LoginSettingsDialog.vue` |

### Dashboard

| 子模块 | 用途 |
|---|---|
| `Dashboard.vue` | 仪表盘主页（widget 网格） |
| `widgets/` | 仪表盘小部件（在线好友 / 最常去世界 / 当前位置 / 通知 / ...） |
| `components/` | 仪表盘组件 |

### Feed / FriendLog / GameLog

| 页面 | 用途 |
|---|---|
| `Feed/Feed.vue` + `columns.jsx` | 综合动态时间轴 |
| `FriendLog/FriendLog.vue` + `columns.jsx` | 好友日志（上线下线/位置变化/状态变化） |
| `GameLog/GameLog.vue` + `columns.jsx` + `components/` + `sessions/` | 游戏日志（玩家进出/视频播放/聊天/截图） |

### FriendList / FriendsLocations / PlayerList

| 页面 | 用途 |
|---|---|
| `FriendList/FriendList.vue` + `columns.jsx` | 完整好友列表（表格视图） |
| `FriendsLocations/FriendsLocations.vue` + `components/` | 好友位置图（按世界分组堆叠头像） |
| `PlayerList/PlayerList.vue` + `columns.jsx` + `components/` + `dialogs/` | 当前房间玩家雷达 |

### Notifications / Moderation

| 页面 | 用途 |
|---|---|
| `Notifications/Notification.vue` + `columns.jsx` + `dialogs/` | 通知中心 |
| `Moderation/Moderation.vue` + `columns.jsx` | 屏蔽/静音管理 |

### Search / MyAvatars / Charts

| 页面 | 用途 |
|---|---|
| `Search/Search.vue` + `components/` + `composables/` | 全局搜索（用户/世界/头像/群组） |
| `MyAvatars/MyAvatars.vue` + `columns.jsx` + `components/` + `composables/` + `ManageTagsDialog.vue` | 我的头像（穿戴/上传/标签管理） |
| `Charts/` + `components/` + `composables/` + `graphLayoutWorker.js` | 数据统计（图表 + 拓扑图） |

### Favorites（4 个分类）

| 页面 | 用途 |
|---|---|
| `Favorites/FavoritesFriend.vue` | 收藏的好友 |
| `Favorites/FavoritesWorld.vue` | 收藏的世界 |
| `Favorites/FavoritesAvatar.vue` | 收藏的头像 |
| `Favorites/components/` + `composables/` + `dialogs/` | 通用组件、组合式函数、对话框（如分组管理） |

### Settings（最大）

| 部分 | 内容 |
|---|---|
| `Settings.vue` | 主框架 |
| `components/Tabs/` | 各 tab 内容 |
| `components/SettingsGroup.vue` / `SettingsItem.vue` / `SimpleSwitch.vue` | 设置项布局 |
| `components/PhotonSettings.vue` | Photon 网络嗅探设置 |
| `components/WristOverlaySettings.vue` | VR 手腕叠加设置 |
| `dialogs/` | 11 个设置子对话框： AvatarProvider / Changelog / FeedFilters / LaunchOptions / NotificationPosition / OpenSourceSoftwareNotice / PrimaryPassword / TranslationApi / VRChatConfig / YouTubeApi |

### Tools

| 页面 | 用途 |
|---|---|
| `Tools/Tools.vue` | 工具页主框架 |
| `Tools/Gallery.vue` | VRC+ 图库管理 |
| `Tools/ScreenshotMetadata.vue` | 截图元数据查看（VRChat 截图嵌入的玩家信息） |
| `Tools/components/` + `dialogs/` | 子组件、对话框 |

---

## 12. VR 模式

独立 entry：`vr.html` + `vr/Vr.vue` + `vr/vr.js` + `vr/components/` + `vr/vr.css`

VrcDog 在 SteamVR 里渲染了独立的叠加层（OpenVR API），用户能在 VR 里看好友列表、通知、当前实例。我们已实现 OvrApi 的 Tauri 后端，前端 VR 入口可参考。

---

## 13. Workers（后台计算）

| Worker | 用途 |
|---|---|
| `activityWorker.js` | 活动数据计算（热力图桶 / 重叠度 / 最常去世界） |
| `activityWorkerRunner.js` | Worker 入口（避免主线程卡顿） |

---

## 14. 国际化

VrcDog 支持 14 种语言：

```
cs / en / es / fr / hu / ja / ko / pl / pt / ru / th / vi / zh-CN / zh-TW
```

`localization/index.js` 是入口，`locales.js` 定义可用语言列表。每种语言一个 JSON 文件（结构与 en.json 完全一致）。

我们的复刻：登录页已实现 14 种切换器，3 种已翻译（zh-CN / en-US / ja-JP），其余 fallback 到 en-US。

---

## 15. 后端 IPC（Tauri 命令映射）

VrcDog 用 .NET（Windows）或 Electron 桥接到原生层。我们用 Tauri 2 + Rust 替代。下表是必须的后端能力映射：

| VrcDog 能力 | VrcDog 调用方式 | 我们的 Tauri 命令 | 状态 |
|---|---|---|---|
| HTTP（cookie 自动带） | `WebApi.execute` (.NET) | `vrc_execute` | ✅ |
| Cookie 持久化 | `AppApi.GetVRChatCookies` | `db_save_auth` / `db_get_auth` | ✅ |
| SteamVR 注册自启动 | `AppApi.RegisterSteamVR` | `sys_register_steamvr_autostart` | ✅ |
| 注册表读写（VRChat config） | `AppApi.GetVRChatRegistryKey` 等 | `sys_get_vrc_config` / `sys_save_vrc_config` | ✅ |
| 启动 VRChat | `AppApi.LaunchVRChat` | `sys_launch_vrchat` | ✅ |
| OSC 发送 | `AppApi.SendOscMessage` | `sys_send_osc_param` / `sys_send_osc_chatbox` | ✅ |
| Discord RPC | `Discord.SetDiscordRichPresence` | `sys_set_discord_rpc` | ✅ |
| OpenVR 叠加层 | C# OVR 包装 | `ovr_*` 系列命令（已 14 个） | ✅ |
| 截图监视 | C# FileSystemWatcher | `gallery_get_images` | ✅ |
| 游戏日志 | C# log 文件解析 | `vrc_get_latest_gamelogs` | ✅ |
| SQLite | EF Core | `rusqlite`（直接） | ✅ |
| 系统通知 | .NET ToastNotification | `tauri-plugin-notification`（todo） | ⚠️ 待接 |
| 文件下载 | `AppApi.DownloadFile` | `bili_download_video` 等 | ✅ |
| URL Scheme（vrcdog:// vrchat://） | C# URI handler | `sys_register_url_scheme` + 启动参数 | ✅ |
| 主密码加密 | C# Aes | `tauri-plugin-stronghold`（todo） | ⚠️ 待接 |

---

## 16. 复刻路线图（按优先级）

> **重要修订（v4.0，2026-05-17）**：本节以前是按 VrcDog 的目录结构（`components/dialogs/UserDialog/` 19 个文件等）描述阶段的，与 vrcdog 实际架构（`components/*View.vue` 平铺 + 单文件 `UserProfileModal.vue`）不一致，导致"已完成"标记产生歧义。本节现已重写为 **以功能为单位** 的路线图，状态严格对齐 `src/` 真实代码。架构层差异（views/coordinators/services 分层）单独列在阶段 **Z** 里讨论。

### 状态图例

- ✅ 已实现（功能在 `src/` 中可见、可跑通）
- 🟡 部分实现（核心可用，但 VrcDog 同位功能还有缺口）
- ⏳ 未实现
- 🛠 仅后端就绪（Tauri 命令存在，前端 UI 缺）

### 阶段总表

| 阶段 | 功能范围 | 真实承载文件 | 状态 |
|---|---|---|---|
| **Login** | 登录 + 2FA + 已保存账号 + 14 国语言菜单 + 自定义 API URL + 代理 | `LoginView.vue` (1081 行) + `authStore.ts` | ✅ |
| **Role / Mode** | 客户端/服务端选择 + PC/VR 模式选择 + 服务端面板 | `RoleSelectView.vue` / `ModeSelect.vue` / `ServerDashboardView.vue` | ✅ |
| **A — UserDialog** | 用户详情面板（Header + Info + Mutual + Groups + Worlds + FavWorlds + Avatars + Activity + 编辑器子对话框） | `UserProfileModal.vue` (2647 行) + `userProfile.ts` | 🟡 见 A 详细 |
| **B — Friends** | 好友列表表格 + 好友位置图 + Sidebar 好友区 | `FriendsListView.vue` / `FriendLocationsView.vue` / `FriendItem.vue` | 🟡 见 B 详细 |
| **C — Feed** | 综合动态（friendLog + gameLog 合并） | `FeedView.vue` (426 行) | 🟡 |
| **D — GameLog** | 游戏日志（玩家进出/视频/聊天） | 内嵌于 `FeedView.vue`，无独立 GameLogView | 🟡 |
| **E — Notifications** | 通知中心（好友请求/邀请/群组通知） | `NotificationsView.vue` (230 行) + `notificationEngine.ts` | 🟡 |
| **F — PlayerList** | 当前房间玩家雷达 | `PlayerListView.vue` (250 行) | 🟡 |
| **G — Search** | 全局搜索（用户/世界/头像/群组） + 全局快搜弹窗 | `SearchView.vue` + `GlobalSearchModal.vue` | 🟡 |
| **H — Charts** | 数据统计（热力图 / 趋势 / Top 世界） | `ChartsView.vue` (639 行) + `HeatmapView.vue` | 🟡 |
| **I — Favorites** | 4 分类收藏夹 + 本地补充收藏 | `FavoritesView.vue` (267 行) | 🟡 |
| **J — Groups Management** | 群组列表 + 群组对话框 + 成员审核 | `GroupsView.vue` (400 行) + `EntityDetailModals.vue` 群组段 | 🟡 |
| **K — WorldDialog** | 世界详情 + 实例 + 标签 + 收藏 | `EntityDetailModals.vue` 世界段 | 🟡 |
| **L — AvatarDialog** | 头像详情 + 风格 + 标签 | `EntityDetailModals.vue` 头像段 | 🟡 |
| **M — InviteDialog** | 邀请系统（发送/请求/模板/历史） | `UserProfileModal.vue` 已支持发送邀请、请求邀请、4 槽消息编辑/选择；图片邀请与独立历史弹窗待补 | 🟡 |
| **N — MyAvatars** | 我的头像（穿戴/上传/标签） | `MyAvatarsView.vue` (147 行) | 🟡 |
| **O — Moderation** | 玩家管理（屏蔽/静音） | `ModerationView.vue` (246 行) | 🟡 |
| **P — Tools** | 工具页（Gallery / ScreenshotMetadata / 邀请模板 / 群组日历 / 数据导出） | `ToolsView.vue` (447 行) + `GalleryView.vue` + `ExportView.vue` | 🟡 |
| **Q — NewInstance / PreviousInstance** | 创建实例 + 历史实例 | `DirectOpenModal.vue` 部分实现 | 🟡 |
| **R — Settings** | 完整设置页（11 个子对话框/分类） | `SettingsView.vue` (1589 行) | 🟡 |
| **S — Dashboard** | 仪表盘 widget（可编辑布局） | `DashboardView.vue` (315 行)（固定布局） | 🟡 |
| **T — VR Overlay** | VR 叠加层（手腕面板/通知/翻译） | `VrLayout.vue` + `OverlayView.vue` + `OvrAdvPanels.vue` (909 行) + `OvrAdvVrDashPanels.vue` + `OvrTranslatorView.vue` (2595 行) | ✅ 大部分 |
| **U — vrcdog 专属** | Bilidown / 远程协助 / 翻译器 / 状态预设 / 角色选择 / 服务端 | `BilidownView.vue` / `RemoteAssistView.vue` / `TranslatorView.vue` / `StatusPresetsView.vue` 等 | ✅ |
| **V — 笔记 / 备忘** | 用户备注 + 本地备忘 | `NotesView.vue` + 用户面板内联编辑 | ✅ |
| **W — i18n 完整化** | 14 种语言完整翻译（当前 zh-CN/en-US/ja-JP 真实，其余 10 种为同一份英文骨架） | `src/i18n/locales/*.json` | 🟡 见 W 详细 |
| **X — 环境管理** | Unity Hub / Unity 2022 / VCC / ALCOM 一键安装 | `EnvView.vue` (662 行) | ✅ |
| **Y — API 模块对齐** | 对齐 VrcDog 22 个 API 模块 | `src/api/*.ts` 已补 avatarModeration / inviteMessages / inventory / prop / misc / VRC+ 等主要模块；queryRequest 仍待独立化 | 🟡 见 Y 详细 |
| **Z — 架构层迁移** | 将 `components/*View.vue` 平铺迁到 `views/Xxx/` + 抽离 `coordinators/` / `services/` 层 | 全工程 | ⏳ 待评估 |

### A — UserDialog 详细状态

VrcDog 在 `components/dialogs/UserDialog/` 里有 19 个文件，vrcdog 用单一 `UserProfileModal.vue` (2647 行) + `userProfile.ts` (Pinia store) 承载。功能不是按文件而是按面板分块的。

| 子功能 | 状态 | 真实承载 |
|---|---|---|
| 用户头像 + profilePicOverride 优先 + 状态点 | ✅ | `UserProfileModal.vue` Header |
| DisplayName + 信任色 + Username 副名 | ✅ | Header |
| 国旗 + 平台徽章（PC/Quest/iOS） | ✅ | Header |
| Trust 徽章 / 18+ / 共同好友 / Discord 等 | 🟡 | Header（部分 VrcDog 标记不全） |
| 曾用名下拉（previousDisplayNames） | ✅ | Header 已显示历史名称下拉 |
| VRChat 官方徽章 + popover + 隐藏/展示开关 | ✅ | Header badges popover + `updateBadge` |
| Bio 翻译按钮 | 🟡 | OvrApi.translate 后端就绪，按钮看实现 |
| bioLinks favicon 列表 | 🟡 | `socialLinks` computed 解析 bio 文本，VrcDog 是 bioLinks 数组字段 |
| userIcon 独立全屏预览 | ✅ | Header 右侧 userIcon 点击预览 |
| More 菜单（自己 vs 他人双语境） | 🟡 | 邀请/请求邀请/群组邀请/编辑器动作已接入，仍有部分高级动作待补 |
| **Info Tab — 当前实例卡片** | 🟡 | 基础实现 |
| **Info Tab — 房间玩家头像列表** | ⏳ | 未实现 |
| **Info Tab — 本地 Note + Memo** | ✅ | `localNote` 内联保存到 SQLite |
| **Info Tab — 当前模型信息（异步取名字）** | ⏳ | 未实现 |
| **Info Tab — 代表群组卡** | ✅ | `getRepresentedGroup` + UI 卡片 |
| **Info Tab — Bio + bioLinks 翻译** | 🟡 | bio 文本可见，翻译按钮待补 |
| **Info Tab — 见面次数 / 一起时长** | ✅ | `db_get_friend_logs` 聚合 |
| **Info Tab — 加好友时间** | ✅ | `db_get_friend_logs` 最早 FriendAdd |
| **Info Tab — 在线/离线时长** | 🟡 | last_login/last_activity 差值 |
| **Info Tab — 加入日期 (date_joined)** | ✅ | base info 直出 |
| **Info Tab — 自己 4 开关**（allowAvatarCopying / isBoopingEnabled / hasSharedConnectionsOptOut / hasDiscordFriendsOptOut） | ✅ | More 菜单动作已接 `saveCurrentUser` |
| **Info Tab — 出生点 homeLocation + 删除** | ✅ | 自己信息区显示 homeLocation 并可清空 |
| **Info Tab — ID 三选一复制下拉** | ✅ | ID / URL / DisplayName 复制 |
| **Mutual Friends Tab**（搜索 + 3 排序） | 🟡 | `mutualFriends` 拉取了，UI 排序待补 |
| **Groups Tab**（拥有/共同/其他三段 + 排序） | 🟡 | `groups` + `mutualGroups` 拉取了，三段分组逻辑待补 |
| **Worlds Tab**（网格 + 排序 + 顺序切换） | 🟡 | `createdWorlds` 有，排序 UI 待补 |
| **Favorite Worlds Tab**（看自己） | 🟡 | `fetchFavoriteWorlds` 有 |
| **Avatars Tab**（创建的模型） | 🟡 | `createdAvatars` 有 |
| **Activity Tab — 时段选择 7/30/90 天** | 🟡 | 看 HeatmapView 复用 |
| **Activity Tab — 真实 SQLite 热力图** | 🟡 | `db_get_heatmap` 已就绪 |
| **Activity Tab — 每日游戏时长** | 🟡 | game logs 解析已就绪 |
| **Activity Tab — 最常去的世界** | 🟡 | game logs 聚合已就绪 |
| **Activity Tab — 排除主世界开关** | ✅ | `excludeHomeWorld` |
| **Activity Tab — 重叠度计算（仅看他人）** | ⏳ | VrcDog 高级功能，未实现 |
| **JSON Tab** | ✅ | `JsonTree.vue` 已挂入用户面板 |
| **导航历史（面包屑）** | ✅ | `navHistory` + `goBack`（vrcdog 自创，VrcDog 没有） |
| **A8a — BioDialog**（512 字 + bioLinks 数组） | ✅ | `UserProfileModal.vue` 内联编辑器，功能等价 |
| **A8a — EditNoteAndMemoDialog** | 🟡 | 内联编辑了，但没独立 dialog |
| **A8a — PronounsDialog** | ✅ | 内联编辑器，接 `saveCurrentUser` |
| **A8a — SocialStatusDialog**（4 状态单选） | ✅ | 内联编辑器，接 `saveCurrentUser` |
| **A8a — LanguageDialog**（多选最多 3 个） | ✅ | 内联编辑器，接 language tags |
| **A8b — SendInviteDialog** | ✅ | 内联 4 槽消息选择/编辑，发送 `messageSlot` |
| **A8b — SendInviteRequestDialog** | ✅ | 内联 4 槽消息选择/编辑，发送 `requestSlot` |

### B — Friends 详细状态

| 子功能 | 状态 | 真实承载 |
|---|---|---|
| 好友列表表格视图 | ✅ | `FriendsListView.vue` (412 行) |
| 好友位置图（按世界堆叠头像） | ✅ | `FriendLocationsView.vue` (368 行) |
| Sidebar 好友区（按状态分组折叠） | 🟡 | 当前 sidebar 是路由按钮列表，未含好友段 |
| 好友卡片（FriendItem 头像/状态环/平台/位置） | ✅ | `FriendItem.vue` (124 行) |
| 搜索 / 排序 / 视图切换 | 🟡 | 看实际代码 |
| 启动时全量同步 + 增量 | ✅ | `authStore.syncInitialFriends` |

### W — i18n 完整化（核心缺口）

| 语言 | 文件大小 | key 数 | 状态 |
|---|---|---|---|
| zh-CN | 103 KB | 2184 | ✅ 真实 |
| en-US | 131 KB | 1566 | ✅ 真实 |
| ja-JP | 136 KB | 1444 | ✅ 真实 |
| es / fr / hu / ko / pl / pt / ru / th / vi / zh-TW | **全部 13.5 KB / 272 key / MD5 完全相同** | 272 | ⏳ **同一份英文骨架，未翻译** |

> 这 10 个文件是同一份英文 stub（MD5 = `97b20323503372e78f1bf830bb3fa4a4`），只是文件名挂着 10 种语种。需要按 zh-CN 的 2184 key 集合补全真实翻译。

### Y — API 模块对齐

VrcDog 有 22 个 API 模块，vrcdog 当前 13 个：

| VrcDog | vrcdog 状态 |
|---|---|
| auth / user / friend / world / avatar / group / notification / favorite | ✅ 已对齐 |
| file / request | ✅ 已对齐（vrcdog 多了 `gamelogWatcher` / `websocket` / `index` 聚合） |
| instance | 🟡 内嵌在 `VrcApi.getInstance` 里，未独立模块 |
| playerModeration | 🟡 用 `getModerations / moderateUser / unmoderateUser` 三个直方法 |
| image | 🟡 `uploadVrcPlusImage` 内嵌 |
| avatarModeration / inviteMessages / prop / inventory / vrcPlusIcon / misc | ✅ 已补主要封装 |
| queryRequest | ⏳ 未独立实现 |

### Z — 架构层迁移（评估中）

VrcDog 用 `views/` + `coordinators/` + `services/` + `queries/` + `composables/` 分层；vrcdog 当前是 `components/*View.vue` 平铺 + 业务逻辑混在 `stores/` 里 + 单 `composables/useToast.ts`。

迁移影响面：
- 44 个 `*.vue` 文件移动 + 改 import
- 业务逻辑从 `authStore` / `userProfile` / `entityModal` 等 store 抽到 `coordinators/`
- HTTP/WebSocket/SQLite 抽到 `services/`
- 工程量大，**收益是可读性与可测试性**，不是新功能

是否做、何时做、做到什么粒度，由用户决定。**默认不做**，先把功能层补齐。

### 真实数据接口映射

| 数据 | 后端 |
|---|---|
| 用户基本信息 | `VrcApi.getUser({ userId })` |
| 好友列表 | `VrcApi.getFriends({ n, offset, offline })` |
| 共同好友 | `VrcApi.getMutualFriends({ userId })` |
| 共同群组 | `VrcApi.getMutualGroups({ userId })` |
| 用户群组 | `VrcApi.request('/users/{id}/groups')` |
| 代表群组 | `VrcApi.getRepresentedGroup({ userId })` |
| 用户世界 | `VrcApi.request('/worlds', { params: { userId } })` |
| 收藏世界 | `VrcApi.getFavoriteWorlds({ userId })` |
| 用户模型 | `VrcApi.request('/avatars', { params: { userId } })` |
| 加入次数 / 一起时长 / 上次见到 | `DbApi.getFriendLogs` 聚合 |
| 热力图 | `DbApi.getHeatmap` |
| 每日游戏时长 | `DbApi.getGameLogs` 按日聚合 |
| 最常去的世界 | `DbApi.getGameLogs` 按 worldId 聚合 |
| 通知 | `VrcApi.getNotifications` + 本地 `DbApi.getNotifications` |
| 实例信息 | `VrcApi.getInstance` |
| 群组成员 | `VrcApi.getGroupMembers` |
| GitHub Release | `https://api.github.com/repos/KingXiaoTaoOVO/VrcDog/releases` |

### 真实数据接口映射

| 数据 | 后端 |
|---|---|
| 用户基本信息 | `VrcApi.getUser({ userId })` |
| 好友列表 | `VrcApi.getFriends({ n, offset, offline })` |
| 共同好友 | `VrcApi.getMutualFriends({ userId })` |
| 用户群组 | `VrcApi.getGroups({ userId })` |
| 用户世界 | `VrcApi.getWorlds({ userId })` |
| 收藏世界 | `VrcApi.getFavoriteWorlds({ userId })` |
| 用户模型 | `VrcApi.getAvatars({ userId })` |
| 加入次数/一起时长/上次见到 | `db_get_friend_logs` 聚合 |
| 热力图 | `db_get_heatmap` |
| 每日游戏时长 | `db_get_game_logs` 按日聚合 |
| 最常去的世界 | `db_get_game_logs` 按 worldId 聚合 |
| 通知 | `VrcApi.getNotifications` + 本地 `db_get_notifications` |
| 实例信息 | `VrcApi.getInstance` |
| 群组成员 | `VrcApi.getGroupMembers` |
| GitHub Release | `https://api.github.com/repos/KingXiaoTaoOVO/VrcDog/releases` |

---

## 17. 当前进度

> **重要修订（v4.0，2026-05-17）**：以前版本声称"UserDialog 整体 ~98% 完工"，是按 VrcDog 文件结构对照得出的。重新审计 `src/` 后发现：vrcdog 是**单文件 `UserProfileModal.vue` (2647 行) + `userProfile.ts` (Pinia store)** 承载，与 VrcDog 的 19 文件分层差异巨大，原"已完成"标记有歧义。本节按真实文件 + 真实功能重新统计。

### 已确认实现（基于实际代码 grep）

**入口与生命周期**
- `LoginView.vue` (1081 行) — 登录 + 2FA + 已保存账号一键登录 + 14 国语言菜单 + 自定义 API URL + 代理设置 + GitHub Releases 更新检查
- `RoleSelectView.vue` (308 行) — 客户端/服务端角色选择
- `ServerDashboardView.vue` (958 行) — 服务端管理面板
- `ModeSelect.vue` — PC/VR 模式选择
- `authStore.ts` — 心跳保活 + 自动登录 + 服务端 ban/kick/freeze 监听 + 好友/通知冷启动同步
- F12 / Ctrl+Shift+I / 右键菜单 生产环境屏蔽
- vrcdog:// / vrchat:// URL Scheme 启动参数处理

**用户面板（A 阶段）**
- `UserProfileModal.vue` + `userProfile.ts`：用户基本信息、共同好友、共同群组、加入的群组、创建的世界、收藏的世界、创建的形象、本地备注、收藏切换、bio 社交链接解析（twitter/x/youtube/twitch/discord/github/patreon）、活动日志、导航历史面包屑（vrcdog 自创，VrcDog 没有）
- 乐观加载（先读 `db_api_cache`，再后台拉取覆盖）

**实体面板**
- `EntityDetailModals.vue` (544 行) — 世界详情 / 群组详情 / 形象详情 三合一
- 群组管理员视图：成员列表、角色、加入申请审批、权限检查
- `entityModal.ts` 收藏切换接 SQLite (`db_add_favorite_world` / `db_add_favorite_avatar`)

**视图（按 sidebar 分类）**
| Tab key | 文件 | 行数 | 功能概述 |
|---|---|---|---|
| dashboard | `DashboardView.vue` | 315 | 在线好友 / 活跃实例 / 缓存计数 / 周热力图 |
| feed | `FeedView.vue` | 426 | 综合动态时间轴 |
| locations | `FriendLocationsView.vue` | 368 | 好友按世界堆叠头像图 |
| charts | `ChartsView.vue` | 639 | 数据统计图表 |
| playerlist | `PlayerListView.vue` | 250 | 当前房间玩家雷达 |
| gallery | `GalleryView.vue` | 365 | 截图图库 |
| social / friendslist | `FriendsListView.vue` | 412 | 好友列表表格 |
| moderation | `ModerationView.vue` | 246 | 屏蔽/静音管理 |
| search | `SearchView.vue` | 308 | 全局搜索（用户/世界/头像/群组） |
| notifications | `NotificationsView.vue` | 230 | 通知中心 |
| groups | `GroupsView.vue` | 400 | 群组列表 + 详情 |
| avatars | `MyAvatarsView.vue` | 147 | 我的头像 |
| favorites | `FavoritesView.vue` | 267 | 收藏夹（4 分类） |
| heatmap | `HeatmapView.vue` | 252 | 独立热力图视图 |
| notes | `NotesView.vue` | 163 | 用户备注列表 |
| presets | `StatusPresetsView.vue` | 274 | 状态预设管理 |
| tools | `ToolsView.vue` | 447 | 工具页 |
| bilidown | `BilidownView.vue` | 790 | B 站视频下载（vrcdog 专属） |
| translator | `TranslatorView.vue` | 662 | 翻译器 |
| ovr | `OvrTranslatorView.vue` | 2595 | OVR 实时翻译（核心专属） |
| remote | `RemoteAssistView.vue` | 620 | 远程协助（vrcdog 专属） |
| env | `EnvView.vue` | 662 | Unity / VCC / ALCOM 环境管理 |
| export | `ExportView.vue` | 179 | 数据导出 |
| settings | `SettingsView.vue` | 1589 | 完整设置页 |

**VR 叠加层（T 阶段，独立 entry）**
- `VrLayout.vue` / `OverlayView.vue` / `OvrAdvPanels.vue` (909 行) / `OvrAdvVrDashPanels.vue` (362 行) — SteamVR 手腕面板 + VR Dashboard

**全局基础设施**
- `i18n/index.ts` — 14 种语言菜单已挂载（vue-i18n）
- `theme.ts` + 主题切换 + 服务端可控的主题白名单
- `notificationEngine.ts` — 通知触发引擎
- `systemContext.ts` — VRChat / SteamVR 进程状态轮询
- `ToastContainer.vue` + `useToast` composable
- `DebugConsole.vue` (246 行) — 内置调试控制台（监听 `app-debug-log` 事件，自动脱敏 password / authCookie / Authorization）
- `GlobalSearchModal.vue` — Ctrl+K 全局快搜
- `DirectOpenModal.vue` — 直接打开实例对话框

**API 层**
- 13 个模块：auth / avatar / favorite / file / friend / gamelogWatcher / group / index / notification / request / user / websocket / world
- `VrcApi` 聚合对象兼容旧调用，`DbApi` 暴露 SQLite，`SysApi` 暴露 Tauri 系统命令，`OvrApi` 暴露 OpenVR 接口，`GalleryApi` / `GamelogApi` 单独命名

### 与 VrcDog 对照下的明确缺口

| 功能 | 状态 | 说明 |
|---|---|---|
| **i18n 完整化** | 🔴 关键 | 10 个语言文件全是同一份 272 key 英文骨架（MD5 相同），需补完 |
| **UserDialog 子对话框（A8a）** | ✅ | Bio / Pronouns / SocialStatus / Language 已以内联编辑器实现 |
| **UserDialog 邀请对话框（A8b）** | ✅ | SendInvite / SendInviteRequest 已以内联 4 槽消息编辑器实现 |
| **曾用名下拉** | ✅ | UserDialog Header 已补 |
| **VRChat 官方徽章 popover** | ✅ | UserDialog Header 已补 |
| **JSON Tab** | ✅ | `JsonTree.vue` 已挂入用户面板 |
| **Sidebar 好友区** | ⏳ | 当前 sidebar 仅路由按钮，无好友按状态分组折叠 |
| **Dashboard 可编辑布局** | ⏳ | 当前固定，VrcDog 是 widget 拖拽 |
| **邀请消息模板管理** | ✅ | `UserProfileModal.vue` 支持 invite/request 发送时编辑；`ToolsView.vue` 支持 message/request/response/requestResponse 四类 4 槽管理 |
| **Photon 嗅探设置** | ⏳ | VrcDog 高级功能 |
| **Discord RPC 详细映射** | 🟡 | 后端 `setDiscordRpc` 在，UI 未做类型映射 |
| **注册表备份** | ⏳ | VrcDog 工具页有 |
| **VRC+ 图库管理** | 🟡 | `gallery_get_images` 后端在，Gallery.vue 看实现 |
| **群组日历** | ⏳ | VrcDog 有月历视图 |
| **截图元数据查看** | ⏳ | VrcDog 有 |
| **拓扑图（共同好友 graph）** | ⏳ | VrcDog 用 force layout |
| **缺失 API 模块** | 🟡 | 主要模块已补；queryRequest 仍待独立化 |

### 下一步建议（按优先级）

1. **W — i18n 补全**：10 个语言文件按 zh-CN 的 2184 key 集合补成真实翻译（一种语言一轮，避免单次输出过长）
2. **Sidebar 好友区**：按状态分组折叠
3. **Dashboard 可编辑布局**：widget 拖拽/持久化
4. **群组日历 / 截图元数据 / 注册表备份**：按 Tools 缺口继续补
5. **Notification invite response UX**：通知中心中直接选择 response/requestResponse 槽位回复
6. **架构层 Z**（可选大动作）：评估是否做 `views/` + `coordinators/` + `services/` 分层迁移

### 总目标

按 VrcDog 的功能清单一比一复刻**功能**，不强求文件结构一致。每个功能用真实 VRChat API + 本地 SQLite，不出现假占位数据。当 vrcdog 的实现路径与 VrcDog 不同（例如单文件 vs 19 文件），以**功能等价**为准。

---

**本文档版本**：3.0（2026-05，UserDialog 整体完工）
**已完成阶段**：Login + A1 + A2 + A3 + A4 + A5 + A6 + A7 + A8a
**下一轮工作**：A8b 邀请对话框 → B 好友列表 / C 动态时间轴


---

# 附录 A：每个视图（View）的深度架构描述

下面把第 11 节列出的 18 个视图全部展开，每个视图描述其：组件树、数据流、用户操作、UI 元素、复刻指引。

## A.1 Dashboard（仪表盘）

### 文件结构
```
views/Dashboard/
├── Dashboard.vue                       # 主页（grid 布局）
├── components/
│   ├── DashboardEditToolbar.vue        # 编辑模式工具栏（保存 / 取消 / 重置）
│   ├── DashboardPanel.vue              # 单个 widget 面板包装（含拖拽手柄、删除按钮）
│   ├── DashboardRow.vue                # 行容器（支持每行不同高度）
│   ├── PanelSelector.vue               # 添加 widget 时的 widget 类型选择器
│   └── panelRegistry.js                # widget 类型注册表（id ↔ 组件映射）
└── widgets/
    ├── FeedWidget.vue                  # 动态时间轴（最近 N 条事件）
    ├── GameLogWidget.vue               # 游戏日志（最近 N 条）
    ├── InstanceWidget.vue              # 当前/上次实例信息卡
    └── WidgetHeader.vue                # widget 通用头（标题 + 操作按钮）
```

### 数据流
- 配置存储在 `dashboardStore.value`（Pinia），结构 `{ rows: [{ height, panels: [{ id, type, props }] }] }`
- 持久化到 `configRepository.setString('VrcDog_dashboardLayout', JSON.stringify(...))`
- 编辑模式开关 `dashboardStore.editing`，切到 true 时 panel 上出现拖拽手柄和删除按钮
- widget 数据来源于其他 store：`feedStore.feedTable` / `gameLogStore.gameLogTable` / `locationStore.lastLocation` 等

### 用户操作流
1. 默认显示已保存的 layout
2. 点 "编辑布局" → DashboardEditToolbar 出现
3. 点 "+" 按钮 → PanelSelector 弹窗，列出可用 widget 类型
4. 拖拽 widget → 改顺序，可跨行
5. 点 widget 右上角 "X" → 移除
6. 点 "保存" → 持久化；点 "取消" → 恢复
7. 点 "重置" → 回到默认 layout

### UI 元素清单（每个 widget）
- `WidgetHeader`：左侧标题 / 中间状态点 / 右侧操作按钮（刷新、隐藏、设置）
- 内容区：滚动容器，按 widget 类型渲染
- 编辑态：覆盖层 + 拖拽手柄 + 删除 X

### 我们项目的现状
我们已有 `DashboardView.vue`，但是固定布局非可编辑。复刻时新增：
- `panelRegistry` 数组定义可选 widget 类型
- DashboardPanel 包装层支持拖拽（用 vue-draggable）
- 把当前 DashboardView 拆分成 widget 形式：在线好友 / 活跃实例 / 缓存计数 / 最近动态

---

## A.2 Sidebar（侧边栏）

### 文件结构
```
views/Sidebar/
├── Sidebar.vue                              # 主框架
├── friendsSidebarUtils.js                   # 好友分类、过滤、排序工具
├── groupsSidebarUtils.js                    # 群组分类工具
├── sidebarSettingsUtils.js                  # 侧栏设置持久化
└── components/
    ├── FriendsSidebar.vue                   # 好友区
    ├── FriendItem.vue                       # 单个好友条目（头像 + 名字 + 状态环）
    ├── GroupsSidebar.vue                    # 群组区
    ├── SortableGroupItem.vue                # 可拖拽群组条目
    ├── FavoriteFriendGroupOrderDialog.vue   # 收藏好友分组排序弹窗
    ├── NotificationCenterSheet.vue          # 通知中心抽屉
    ├── NotificationItem.vue                 # 单个通知条目
    └── NotificationList.vue                 # 通知列表容器
```

### Sidebar 主框架的分区
1. **顶部**：当前用户头像 + displayName + 状态点 + 状态描述
2. **导航段**：路由按钮（Dashboard / Feed / FriendList / FriendsLocations / PlayerList / GameLog / FriendLog / Notifications / Search / MyAvatars / Charts / Favorites / Moderation / Tools / Settings）
3. **好友段**：在线好友按状态分组（Active / Join Me / Ask Me / Busy / Offline），每段可折叠
4. **群组段**：当前用户群组列表（按 in-game 顺序）
5. **底部**：通知铃铛 + 未读数 + Settings 入口

### FriendsSidebar 的过滤
`friendsSidebarUtils.js` 包含：
- `groupFriendsByStatus(friends)` — 按 active/join me/ask me/busy/offline 分桶
- `filterByLocation(friends, currentLocation)` — 高亮和你同一实例的好友
- `applySearch(friends, query)` — 模糊匹配 displayName / username

### FriendItem 显示元素
- 头像（圆形，userImage）
- 状态环（trust 色或状态色）
- displayName（trust 色）
- 副信息（在哪个世界 / 离线时长 / "和你同一实例" 标记）
- 平台徽章（PC/Quest/iOS）
- 红点（未读消息）

### NotificationCenterSheet（抽屉）
点铃铛弹出右侧抽屉：
- Tab 切换：全部 / 好友请求 / 邀请 / 群组通知 / 系统通知
- 每个通知 NotificationItem：图标 + 标题 + 描述 + 时间戳 + 操作按钮（接受 / 拒绝 / 查看）
- 底部："清除全部已读" / "标记全部为已读"

---

## A.3 Login（登录页）

### 文件结构
```
views/Login/
├── Login.vue                       # 主页
└── Dialog/
    └── LoginSettingsDialog.vue     # 设置弹窗（代理 + 自定义 API）
```

### 状态阶段（state machine）
1. **idle**：等待用户输入
2. **submitting**：提交中
3. **2fa-required**：弹出 2FA 输入
4. **2fa-submitting**：2FA 验证中
5. **success**：跳转到主页
6. **error**：显示错误（密码错误 / 网络错误 / Cookie 过期）

### Login.vue UI 分区
- **左上工具栏**：齿轮（设置）/ 向下箭头（更新）/ 字母 A（语言菜单 14 种）
- **左侧表单**：用户名输入 / 密码输入 / 保存登录数据 / 登录按钮 / 注册按钮
- **右侧已保存账号**：账号卡片列表（头像 + displayName + username + 删除按钮），点击触发 relogin
- **底部**：忘记密码链接 + 版权声明 + 免责声明
- **左下**：版本号

### LoginSettingsDialog 字段
- 代理地址（HTTP/SOCKS5）
- 自定义 API URL（用于私服或镜像）
- 主密码（如启用了 PrimaryPassword 加密）

### 我们的现状
✅ 已完成（Login + 工具栏 + 已保存账号 + 14 国语言菜单 + 设置弹窗 + 更新检查弹窗）

---

## A.4 Feed（综合动态）

### 文件结构
```
views/Feed/
├── Feed.vue              # 主页
└── columns.jsx           # DataTable 列定义
```

### 数据来源
合并三类源：
- `feedStore.feedTable` — 好友上下线、状态变化、位置变化
- `friendStore.friendLogTable` — 加好友/删好友/改名等关系变化
- `gameLogStore.gameLogTable` — 玩家进出实例

### Feed.vue UI 分区
- **顶部**：搜索框 + filters 按钮（点击开 FeedFiltersDialog）+ 刷新按钮
- **filters 弹窗**：勾选启用哪些事件类型（onlinePlayers / friendOnlines / friendOfflines / userStateChange / locationChange / friendAdd / friendRemove / etc）
- **DataTable**：时间戳 / 类型图标 / 用户头像 / displayName / 描述 / 操作（打开用户对话框 / 复制 ID）
- **分页**：页码 + 每页数量

### columns.jsx 列定义
- `时间`：dayjs 格式化为 "今天 14:30" / "昨天 09:12" / "5月13日 14:30"
- `事件`：图标 + 类型标签
- `玩家`：可点击 → showUserDialog
- `详情`：根据事件类型展开（如位置变化显示 from → to）

---

## A.5 GameLog（游戏日志）

### 文件结构
```
views/GameLog/
├── GameLog.vue                              # 主页
├── columns.jsx                              # 列定义
├── components/
│   ├── GameLogSessions.vue                  # 会话视图（按游戏会话分组）
│   ├── GameLogSessionsEvent.vue             # 单个事件
│   └── GameLogSessionsSegment.vue           # 会话片段（一个实例的时间区间）
└── sessions/
    └── buildGameLogSessions.js              # 把扁平 events 重组为 sessions 的纯函数
```

### 视图模式切换
- **Table 视图**：传统表格（时间 / 类型 / 内容）
- **Sessions 视图**：按 VRChat 启动 → 退出 切分会话，每个会话内按实例切分 segment，每个 segment 内显示玩家进出事件

### 事件类型（VRChat 日志解析）
- `User Authenticated` — 登录
- `Entering Room` — 进入实例
- `OnPlayerJoined` — 玩家加入
- `OnPlayerLeft` — 玩家离开
- `[Video Playback]` — 视频播放（含 URL）
- `[ChatBox]` — 聊天消息
- `Application Quit` — 退出 VRChat

### sessions/buildGameLogSessions.js 算法
输入扁平的 GameLogEvent 数组，输出嵌套：
```
sessions: [
  {
    startTime, endTime,                     # VRChat 启动到退出
    segments: [
      {
        startTime, endTime,                 # 单个实例的时间
        worldId, instanceId, worldName,
        events: [{ type, time, content }]   # 该实例内的玩家进出/视频/聊天
      }
    ]
  }
]
```

### Session 视图的 UI
- 时间轴左侧：每个 session 折叠卡片，点开看 segments
- 每个 segment：世界缩略图 + 名字 + 时长 + 玩家头像列表
- 每个 event：图标 + 时间 + 内容

---

## A.6 FriendList（好友列表）

### 文件结构
```
views/FriendList/
├── FriendList.vue
└── columns.jsx
```

### UI 分区
- **顶部 toolbar**：刷新 / 搜索框 / 排序下拉 / 视图切换（表格/卡片）/ 过滤（在线/离线/全部）
- **DataTable / Grid**：头像 / displayName / 状态 / 当前位置 / 上次见到 / 操作

### columns.jsx 字段
- `头像`：用户图片 + 状态环（trust 色或状态色）
- `displayName`：trust 色，点击 showUserDialog
- `状态`：状态点 + 状态文字 + 状态描述
- `位置`：Location 组件（可点击 showWorldDialog）
- `上次见到`：dayjs 相对时间
- `加入次数`：和你一起在的实例次数
- `一起时长`：累积同实例时间
- `本地备注`：编辑/查看
- `操作`：邀请 / 申请加入 / 复制 ID / 删除好友（下拉菜单）

---

## A.7 FriendLog（好友日志）

### 文件结构
```
views/FriendLog/
├── FriendLog.vue
└── columns.jsx
```

### 事件类型
- `Friend Added` — 加好友
- `Friend Removed` — 删好友
- `Display Name Changed` — 改名（带历史 displayName）
- `Trust Level Changed` — 信任等级变化
- `Status Changed` — active/join me/ask me/busy 切换
- `Location Changed` — 位置变化

### UI
DataTable，列：时间 / 事件类型图标 / 头像 / displayName / 详情（变化前→变化后）/ 操作。

---

## A.8 FriendsLocations（好友位置图）

### 文件结构
```
views/FriendsLocations/
├── FriendsLocations.vue
└── components/
    └── （多个子组件，待挖掘）
```

### 视图
按当前所在世界把好友分组堆叠：
- 每个分组一个卡片：世界缩略图 + 名字 + 实例 ID + 玩家计数 + 推荐人数
- 卡片内部：好友头像堆叠（最多 8 个，超过显示 +N）
- 点头像 → showUserDialog
- 点世界 → showWorldDialog
- 卡片操作：邀请自己加入 / 复制位置链接 / 申请邀请

### 顶部工具栏
- 刷新
- 过滤：在公开/好友+/好友/邀请+/邀请/群组实例
- 隐藏：私密实例（位置不可见）
- 搜索：按世界名搜索
- 排序：按好友数 / 按世界名 / 按推荐人数

---

## A.9 PlayerList（房间玩家雷达）

### 文件结构
```
views/PlayerList/
├── PlayerList.vue
├── columns.jsx
├── components/                  # 玩家卡片、过滤器
└── dialogs/                     # 房间内玩家相关弹窗
```

### 数据来源
当前实例的玩家列表，来自：
- VRChat 日志（`OnPlayerJoined` / `OnPlayerLeft` 流式更新）
- VRChat API `instance.users`（如果实例可见）
- Photon 嗅探（高级功能）

### UI
- 顶部：当前实例信息卡（同 Info Tab 的实例卡片）
- 玩家列表（DataTable 或 Grid）：头像 / displayName / trust / 平台 / 进入时间 / 是否好友 / 是否屏蔽 / 备注
- 操作：showUserDialog / 添加好友 / 屏蔽 / 静音 / 添加本地备注

---

## A.10 Notifications（通知中心）

### 文件结构
```
views/Notifications/
├── Notification.vue
├── columns.jsx
└── dialogs/                # 通知相关弹窗（如群组邀请处理）
```

### 通知分类（VrcDog 标准）
- `friendRequest` — 好友请求
- `invite` — 邀请
- `requestInvite` — 邀请请求
- `inviteResponse` / `requestInviteResponse` — 邀请响应
- `boop` — 戳一戳
- `groupInvite` / `groupJoinRequest` — 群组邀请/加入请求
- `groupAnnouncement` — 群组公告
- `groupTransfer` — 群组所有权转移
- `votetokick` — 群组踢人投票

### UI
- Tab 切换：全部 / 未读 / 各类型
- 列表：图标 + 标题 + 描述 + 时间 + 操作按钮（接受/拒绝）
- 已读/未读切换 + 全部清除

---

## A.11 Moderation（玩家管理）

### 文件结构
```
views/Moderation/
├── Moderation.vue
└── columns.jsx
```

### 数据
`playerModerationStore.playerModerationTable` — 来自 `GET /auth/user/playermoderations`

### UI
- Tab 切换：屏蔽 / 静音 / 显示头像 / 隐藏头像
- DataTable：头像 / displayName / 类型 / 创建时间 / 操作（解除）
- 顶部：搜索 + 批量操作

---

## A.12 Search（全局搜索）

### 文件结构
```
views/Search/
├── Search.vue
├── components/
│   └── SearchPagination.vue
└── composables/
    ├── useSearchUser.js              # 用户搜索逻辑
    ├── useSearchWorld.js             # 世界搜索
    ├── useSearchAvatar.js            # 头像搜索（用 avatarProvider 第三方源）
    └── useSearchGroup.js             # 群组搜索
```

### 搜索类型
- **用户**：`POST /users` 搜索 displayName，结果分页
- **世界**：`GET /worlds?search=X`，可选过滤（platform / featured / sort）
- **头像**：第三方头像数据库（VrcDog 用的是社区源，需要 avatarProviderStore 配置）
- **群组**：`GET /groups?query=X`

### Search.vue UI
- Tab 切换：用户 / 世界 / 头像 / 群组
- 搜索框 + 高级过滤（每种类型不同）
- 网格/列表视图切换
- 分页（SearchPagination）
- 点击结果：showUserDialog / showWorldDialog / showAvatarDialog / showGroupDialog

---

## A.13 MyAvatars（我的头像）

### 文件结构
```
views/MyAvatars/
├── MyAvatars.vue
├── columns.jsx
├── ManageTagsDialog.vue            # 批量管理标签
├── components/                     # 卡片、过滤器
└── composables/                    # 批量操作
```

### UI
- 顶部：刷新 / 搜索 / 排序（更新时间/创建时间/名字）/ 过滤（公开/私密/全部）
- 网格视图：每个卡片显示缩略图 + 名字 + 标签 + 平台支持图标
- 卡片操作：穿戴 / 编辑 / 上传新缩略图 / 设置标签 / 删除
- 批量选择：复选框 + 批量管理标签 / 批量删除

### 卡片字段
- 缩略图（imageUrl）
- name
- description
- releaseStatus（public/private）
- tags 数组
- platforms（standalonewindows/android）
- 上传时间 / 更新时间

---

## A.14 Charts（数据统计）

### 文件结构
```
views/Charts/
├── graphLayoutWorker.js                # 拓扑图布局算法（Web Worker）
├── components/
│   ├── HotWorlds.vue                   # 最常去世界
│   ├── InstanceActivity.vue            # 实例活动趋势
│   ├── InstanceActivityDetail.vue      # 单个实例详情
│   ├── InstanceActivityTooltip.jsx     # 悬浮提示
│   └── MutualFriends.vue               # 共同好友拓扑图
└── composables/
    ├── useActivityDataFilter.js
    ├── useActivityDataProcessor.js
    ├── useActivityStats.js
    ├── useChartHelpers.js
    ├── useDateNavigation.js
    ├── useInstanceActivityData.js
    ├── useInstanceActivitySettings.js
    └── useIntersectionObserver.js
```

### Charts 包含的图表
1. **Hot Worlds** — 你最常去的世界 Top N（柱状图 + 缩略图列表）
2. **Instance Activity** — 你在 VRChat 的活跃趋势（折线/面积图，按日/周/月聚合）
3. **Instance Activity Detail** — 选中某天后展开，显示当天的实例时间分布
4. **Mutual Friends Graph** — 拓扑图（D3 force layout 或自实现），节点=好友，边=共同好友关系
   - graphLayoutWorker.js 负责布局计算（避免阻塞主线程）

### 技术栈
- ECharts 4 渲染所有图表
- Web Worker 计算布局
- IntersectionObserver 懒渲染（图表多时性能优化）

---

## A.15 Favorites（收藏夹）

### 文件结构
```
views/Favorites/
├── FavoritesFriend.vue              # 收藏好友
├── FavoritesWorld.vue               # 收藏世界
├── FavoritesAvatar.vue              # 收藏头像
├── favorites-layout.css
├── components/                      # 卡片、分组组件
├── composables/                     # 拖拽、批量操作
└── dialogs/                         # 收藏夹相关弹窗
```

### VRChat 收藏夹规则
- 好友收藏：4 组 × 50 = 200（VRC+ 4 组 × 50 = 额外 200）
- 世界收藏：4 组 × 100（VRC+ 增加）
- 头像收藏：4 组 × 25（VRC+ 4 组 × 25 = 额外 100）
- 群组收藏：4 组（无 VRC+ 加成？）

### UI（每个分类都类似）
- 顶部：分组 tab（默认 4 组 + VRC+ 4 组）+ 当前组容量（30/100）
- 分组管理：重命名 / 重排序 / 移动条目到其他组
- 网格视图：卡片可拖拽（同组内重排 + 跨组移动）
- 操作：移除 / 跳到详情 / 批量选择

### 本地补充
VrcDog 还在本地维护一个"Local Favorites"分组（突破 VRChat 收藏数量限制），存在 SQLite。

---

## A.16 Settings（设置）

### 文件结构
```
views/Settings/
├── Settings.vue                                   # 主框架（左 nav + 右 tab content）
├── components/
│   ├── PhotonSettings.vue                         # Photon 嗅探设置
│   ├── SettingsGroup.vue                          # 设置组容器（标题 + items）
│   ├── SettingsItem.vue                           # 单个设置项（标签 + 控件 + 描述）
│   ├── SimpleSwitch.vue                           # 简易开关
│   ├── WristOverlaySettings.vue                   # VR 手腕叠加设置
│   └── Tabs/
│       ├── InterfaceTab.vue                       # 界面 tab（外观/字体/主题/导航）
│       ├── SocialTab.vue                          # 社交 tab（好友显示/隐藏备注）
│       ├── NotificationsTab.vue                   # 通知 tab（声音/桌面/TTS/位置）
│       ├── MediaTab.vue                           # 媒体 tab（截图/录像/图库）
│       ├── VrTab.vue                              # VR tab（叠加层/手腕/SteamVR）
│       ├── IntegrationsTab.vue                    # 集成 tab（Discord/OSC/启动参数）
│       ├── AdvancedTab.vue                        # 高级 tab（Photon/翻译/主密码/数据库）
│       └── SystemTab.vue                          # 系统 tab（自启动/日志/重置/关于）
└── dialogs/
    ├── AvatarProviderDialog.vue                   # 头像数据库源管理
    ├── ChangelogDialog.vue                        # 更新日志
    ├── FeedFiltersDialog.vue                      # 动态过滤器配置
    ├── LaunchOptionsDialog.vue                    # VRChat 启动参数
    ├── NotificationPositionDialog.vue             # 桌面通知位置
    ├── OpenSourceSoftwareNoticeDialog.vue         # 开源软件声明
    ├── PrimaryPasswordDialog.vue                  # 主密码（加密保存的凭证）
    ├── TranslationApiDialog.vue                   # 翻译 API 配置
    ├── VRChatConfigDialog.vue                     # 编辑 config.json
    ├── YouTubeApiDialog.vue                       # YouTube API key（视频信息预览）
    └── registryBackupColumns.jsx                  # 注册表备份列定义
```

### Settings.vue 主框架
左侧 nav（每个 tab 一个图标 + 标签）+ 右侧 tab content。Tab 顺序：Interface / Social / Notifications / Media / VR / Integrations / Advanced / System

### InterfaceTab 包含的设置项
- 主题选择（dark / light / pink / blackbarnacle / nature 等多种）
- 字体（11 种 fallback 顺序）
- 信任色覆盖（每个 trust 等级允许自定义颜色）
- 隐藏备注 / 隐藏备忘
- 时间格式（12/24 小时）
- 时区
- 日历周起始日
- displayName 显示模式（trust 色 / 自定义色 / 群组代表色）
- 用户数据表行数限制
- 动画开关
- 表情类型（系统 / animated）

### SocialTab
- 共同好友显示开关（VRChat 服务器端字段）
- Discord 朋友显示开关
- 隐藏备忘文本
- 隐藏备注文本
- Friend Sidebar 排序（活动 / 字母 / 好友顺序）
- Friend Sidebar 是否分组按状态
- Friend Sidebar 离线好友是否显示

### NotificationsTab
- 桌面通知开关 + 位置（NotificationPositionDialog）
- 通知声音 + 自定义音效路径
- TTS 开关 + 引擎 + 音色 + 速率
- 各事件类型通知开关（好友上下线 / 邀请 / 状态变化 / 加好友请求 / ...）
- 桌面通知何时显示（永远 / 桌面模式 / VR 模式 / VRChat 运行 / VRChat 未运行）

### MediaTab
- 截图目录
- 截图监视开关
- 截图加水印（内嵌 VRChat metadata）
- 录像支持
- 图库默认视图

### VrTab
- VR 叠加层开关
- 手腕叠加层开关 + 手（左/右）+ 透明度
- SteamVR 自启动注册
- 触摸板/摇杆切换叠加
- VR 通知动画
- WristOverlaySettings.vue 包含完整手腕配置

### IntegrationsTab
- Discord RPC 开关 + 详情/状态文本 + 各事件类型映射
- OSC 监听端口 / 发送端口 + 静音同步
- VRChat 启动参数（LaunchOptionsDialog）
- URL Scheme 注册（vrcdog://）

### AdvancedTab
- Photon 嗅探（PhotonSettings.vue）
- 翻译 API（TranslationApiDialog）：腾讯/百度/微软/Ollama 自定义
- YouTube API（YouTubeApiDialog）
- 主密码（PrimaryPasswordDialog）：用主密码加密 savedCredentials
- 数据库路径 + 备份/恢复
- 重置缓存 / 重置全部设置

### SystemTab
- 自启动开关
- 启动时最小化
- 关闭到托盘
- 默认页面（路由）
- 调试日志
- 注册表备份（RegistryBackupDialog）
- VRChat config.json 编辑（VRChatConfigDialog）
- 关于（版本 / 贡献者 / 开源软件声明）

### Settings 用到的 store
- `settings/general` / `settings/appearance` / `settings/notifications` / `settings/discordPresence` / `settings/advanced` / `settings/wristOverlay`

---

## A.17 Tools（工具页）

### 文件结构
```
views/Tools/
├── Tools.vue                                # 工具列表主页
├── Gallery.vue                              # VRC+ 图库（独立页面）
├── ScreenshotMetadata.vue                   # 截图元数据查看
├── components/
│   ├── GlobalToolsDialogs.vue               # 全局工具对话框装载
│   ├── GroupCalendarEventCard.vue           # 群组日历事件卡
│   ├── GroupCalendarMonth.vue               # 群组日历月视图
│   └── ToolItem.vue                         # 单个工具入口
└── dialogs/
    ├── AutoChangeStatusDialog.vue           # 自动状态切换（按时间表/事件）
    ├── EditInviteMessageDialog.vue          # 编辑单条邀请消息模板
    ├── EditInviteMessagesDialog.vue         # 邀请消息模板管理（4 类各 4 槽）
    ├── ExportAvatarsListDialog.vue          # 导出头像列表
    ├── ExportDiscordNamesDialog.vue         # 导出好友 Discord 名字
    ├── ExportFriendsListDialog.vue          # 导出好友列表
    ├── GroupCalendarDialog.vue              # 群组日历完整视图
    ├── NoteExportDialog.vue                 # 导出本地备注
    └── RegistryBackupDialog.vue             # 注册表备份（VRChat 设置备份）
```

### Tools 入口列表
- VRC+ 图库
- 截图元数据查看
- 自动状态切换
- 邀请消息模板管理
- 群组日历
- 数据导出（好友/头像/备注/Discord 名）
- 注册表备份

### Gallery（VRC+ 图库）
- 上传图片（PNG/JPG，最大 10MB，VRC+ 用户限定）
- 网格视图，缩略图
- 操作：复制 URL（用于 Bio link）/ 删除 / 下载
- 4 个 gallery slot（VRC+ 用户）

### ScreenshotMetadata
- 拖拽 PNG → 解析嵌入的 VRChat 玩家信息
- 显示：实例 ID / 世界名 / 当时在场玩家列表 / 时间戳

### EditInviteMessagesDialog
- 4 类（message / response / request / requestResponse），每类 4 个 slot
- 每个 slot 可编辑文本 + 上传图片
- 冷却时间显示（VRChat 限制每个 slot 修改后 1h 才能再修改）

### GroupCalendarDialog（月视图）
- 显示用户加入的群组的所有日历事件
- 月历布局，事件占格子
- 点击事件 → GroupCalendarEventCard 弹窗（详情 + 报名）

---

## A.18 MainLayout

### 文件
`views/Layout/MainLayout.vue`

### 结构
```
┌─────────────────────────────────────────────┐
│  Sidebar │   <router-view />                │
│          │                                  │
│  (300px) │   (rest)                         │
│          │                                  │
│          │                                  │
│          │                                  │
└─────────────────────────────────────────────┘
│  StatusBar (底部状态栏，总是显示)            │
└─────────────────────────────────────────────┘
```

### StatusBar 显示
- VRChat 服务器状态点（Operational / Issue）
- WebSocket 连接状态点
- 当前 ping
- VRChat 运行状态
- 当前实例计数（X 玩家 / 5 好友）
- 内存占用 / FPS（开发模式）

---

# 附录 B：每个 Coordinator 的工作流

下面把第 5 节列出的 24 个 coordinator 全部展开。

## B.1 authCoordinator
- `login(form)` — 走 2FA 流程，处理各种错误
- `logout()` — 调用 VRChat logout API + 清空状态
- `relogin(savedCred)` — 用 savedCredentials 重新登录
- `verify2FA(code, method)` — 提交 OTP/TOTP/Email
- `resendEmailOtp()`

## B.2 authAutoLoginCoordinator
- `tryAutoLogin()` — 启动时检查保存的凭证 → 走自动登录
- 失败时降级到缓存的 currentUser（保持本地 UI 可见，但 API 调用会失败）

## B.3 userSessionCoordinator
- `onLogin(user)` — 启动时序：初始化 WebSocket / 拉取 friends / 拉取 notifications / 拉取 favorites / 启动 update loop
- `onLogout()` — 反向清理

## B.4 userCoordinator
- `showUserDialog(userId)` — 打开对话框，触发数据加载
- `refreshUserDialog()` — 刷新当前对话框
- `applyUser(json)` — 把 API 返回的 JSON 转成内部 ref 格式（含 `$xxx` 计算字段）

## B.5 userEventCoordinator
- 监听 WebSocket 事件，分发到对应 store
- `friend-online` → 更新 friendStore + 触发上线通知
- `friend-offline` → 更新 + 触发离线通知
- `friend-location` → 更新 lastLocation
- `friend-update` → diff 状态变化

## B.6 friendSyncCoordinator
- `syncAllFriends()` — `processBulk` 拉取全部好友，分页 100/批次
- `syncOnlineFriends()` — 只拉在线
- `syncOfflineFriends()` — 只拉离线

## B.7 friendPresenceCoordinator
- 计算每个好友的 `$online_for` / `$offline_for`
- 维护 `userActivityTable`（活动时段记录）

## B.8 friendRelationshipCoordinator
- `addFriend(userId)` — POST friend request
- `cancelFriendRequest(userId)`
- `acceptFriendRequest(notificationId)`
- `deleteFriend(userId)` — 含确认对话框

## B.9 worldCoordinator
- `showWorldDialog(location)` — 解析 location → 加载世界 → 加载实例
- `applyWorld(json)`
- `refreshInstancePlayerCount(location)`

## B.10 avatarCoordinator
- `showAvatarDialog(avatarId)`
- `selectAvatar(avatarId)` — 穿戴
- `selectFallbackAvatar(avatarId)` — 设为备用
- `uploadAvatarThumbnail(blob)`

## B.11 groupCoordinator
- `showGroupDialog(groupId)`
- `applyGroup(json)`
- `joinGroup(groupId)` / `leaveGroup(groupId)` / `leaveGroupPrompt(groupId)`
- `setGroupVisibility(groupId, visibility)` — visible/friends/hidden
- `saveCurrentUserGroups()` — 持久化群组列表
- `updateInGameGroupOrder()` — 从 VRChat 注册表读群组顺序

## B.12 instanceCoordinator
- `applyInstance(json)`
- `refreshInstancePlayerCount(location)`
- `getInstance(location)` — 缓存命中或拉取
- 处理 `playerJoiningInstance` 状态（你在传送中）

## B.13 locationCoordinator
- 监听 gameLog 的 "Entering Room" 事件
- 更新 `lastLocation` / `lastLocationDestination`
- 触发 Discord RPC 更新 / 触发 dashboard widget 刷新

## B.14 inviteCoordinator
- `sendInvite(userId, message, image?)` — 走 SendInviteDialog
- `requestInvite(userId, message)`
- `respondToInvite(notificationId, accept)`
- `refreshInviteMessageTableData()` — 刷新 4 类邀请消息模板的数据

## B.15 moderationCoordinator
- `applyPlayerModeration(userId, type)` — block/mute/showAvatar/hideAvatar
- `handlePlayerModerationDelete(moderationId)`
- 同步本地 moderationTable

## B.16 favoriteCoordinator
- `addFavorite(type, favoriteId, tags)` — friend/world/avatar/group
- `removeFavorite(favoriteId)`
- `moveFavoriteBetweenGroups()`
- `renameFavoriteGroup(groupId, newName)`

## B.17 gameCoordinator
- `isVRChatRunning()` — 进程检测
- `launchVRChat(args)` — 启动游戏
- `killVRChat()`
- `clearVRChatCache()` — 清缓存
- 监听 VRChat 启动/退出事件

## B.18 gameLogCoordinator
- 启动 watcher 监视 output_log_xxxx.txt
- 每秒读新行 → 解析成 GameLogEvent → 写入 SQLite + 推到 store
- 检测当前实例（最新的 `Entering Room` 事件）
- 解析聊天消息（`[ChatBox]`）
- 解析视频播放（`[Video Playback]`）

## B.19 dateCoordinator
- 计算 `userOnlineFor(user)` — 在线时长字符串
- 计算 `userOnlineForTimestamp(user)` — 上线时间戳
- 计算 `dateFriended` / `dateFriendedInfo` — 友谊建立日期

## B.20 cacheCoordinator
- 周期性清理过期缓存
- 各种 Map 的 LRU 淘汰
- localStorage / SQLite 中过期数据清理

## B.21 searchIndexCoordinator
- 维护本地搜索索引（用户/世界/头像/群组）
- 在 quickSearchWorker（Web Worker）里跑模糊匹配

## B.22 imageUploadCoordinator
- 多步骤图片上传协议
  1. POST `/file/{id}/{version}/file/start` → 拿到上传 URL
  2. PUT 上传到 S3
  3. POST `/file/{id}/{version}/file/finish` → 确认完成
- 用于头像缩略图、世界缩略图、bioLinks favicon、邀请消息图片等

## B.23 memoCoordinator
- 备忘录 CRUD
- 同步到本地 SQLite

## B.24 vrcdogCoordinator
- 自启动注册
- 系统托盘
- URL Scheme 处理（vrcdog://launch/wrld_xxx）
- VRChat URL 处理（vrchat://launch/wrld_xxx）

---

# 附录 C：Stores 字段精细化

## C.1 useUserStore（user.js）

### 关键 ref
- `currentUser` — 当前登录用户对象（含 `$xxx` 计算字段）
- `cachedUsers` — Map<userId, userRef>，全局用户缓存
- `userDialog` — UserDialog 状态对象
  - `id` — 当前打开的用户 ID
  - `visible` — 是否显示
  - `loading` — 是否加载中
  - `ref` — 用户对象引用
  - `friend` — 好友对象（如果是好友）
  - `isFriend` — boolean
  - `note` / `memo` — 本地备注
  - `joinCount` / `timeSpent` / `lastSeen` — 统计字段
  - `mutualFriends` / `mutualFriendCount` / `isMutualFriendsLoading`
  - `userGroups` — `{ groups, ownGroups, mutualGroups, remainingGroups }`
  - `isGroupsLoading` / `groupSorting`
  - `representedGroup` / `isRepresentedGroupLoading`
  - `instance` — 当前实例引用
  - `users` — 实例内玩家
  - `$location` — 解析后的位置对象
  - `$homeLocationName` — 出生点名字
  - `dateFriended` / `dateFriendedInfo` / `unFriended`
  - `mutualFriendSorting`
  - `lastActiveTab`
  - `activeTab`
  - `previousDisplayNames`

### actions
- `cachedUsers.get(userId)` / `cachedUsers.set(userId, ref)`
- `applyUser(json)` — 转换 + 缓存
- `applyUserLanguage(ref)` — 从 tags 提取 `language_*`
- `toggleSharedConnectionsOptOut()` — VRChat 设置开关
- `toggleDiscordFriendsOptOut()`

## C.2 useGroupStore（group.js）

### 关键 ref
- `groupDialog` — GroupDialog 状态
- `cachedGroups` — Map<groupId, groupRef>
- `currentUserGroups` — 当前用户加入的群组 Map
- `inGameGroupOrder` — 群组在 VRChat 内的排序数组（从注册表读取）
- `inviteGroupDialog` — 群组邀请对话框

### actions
- `applyGroup(json)`
- `showGroupDialog(groupId)`
- `showModerateGroupDialog(groupId)`

## C.3 useFavoriteStore（favorite.js）

### 关键 ref
- `favoriteFriendGroups` — 4 组 + VRC+ 4 组
- `favoriteWorldGroups` — 4 组 + VRC+ 4 组
- `favoriteAvatarGroups`
- `localFavoriteFriends` — 本地补充收藏

### actions
- `showFavoriteDialog(type, id)` — 打开 ChooseFavoriteGroupDialog
- `addFavorite(type, id, tags)`
- `removeFavorite(favoriteId)`

## C.4 useNotificationStore（notification/index.js）

### 关键 ref
- `notificationTable` — 通知列表
- `unseenNotifications` — 未读集合
- `notificationFilters` — 当前过滤器

### actions
- `markAllAsSeen()`
- `clearAll()`
- `applyNotification(json)`

### overlayDispatch.js
处理通知弹出到 VR 叠加层的逻辑：
- 根据通知类型 → 决定显示位置 / 显示时长 / 是否带操作按钮
- 推送给 VR overlay window

## C.5 useGameLogStore（gameLog/index.js）

### 关键 ref
- `gameLogTable` — 解析过的事件列表
- `gameLogSessionTable` — 按 session 分组的视图
- `gameLogIsActive` — 监视器是否运行
- `lastGameLogTime`

### actions
- `startWatcher()` / `stopWatcher()`
- `applyEvent(event)` — 写入 + 触发副作用（feed 推送、通知）

### mediaParsers.js
- `parseYouTubeUrl(url)` — 提取 video ID + 拉 YouTube API 元数据
- `parseTwitchUrl(url)`
- `parseSoundCloudUrl(url)`
- 视频日志的"标题/时长"显示逻辑

## C.6 useFeedStore

合并显示 feed/friendLog/gameLog，按日期分组。

## C.7 useChartsStore

### 关键 ref
- `weeklyTrend` — 周活跃趋势数据
- `topWorlds` — 最常去世界
- `mutualGraph` — 拓扑图节点+边数据
- `instanceActivity` — 实例活动时间序列

### actions
- `buildMutualGraph()` — 用 graphLayoutWorker 计算布局
- `refreshTopWorlds(periodDays)`
- `refreshWeeklyTrend()`

## C.8 useUiStore

### 关键 ref
- `appLanguage` — 当前语言
- `theme` — 当前主题
- `sidebarWidth` — 侧栏宽度
- `shiftHeld` — Shift 键是否按下（影响某些操作的"危险模式"）
- `currentRoute`
- `notificationCenterOpen`
- `quickSearchOpen`

## C.9 useModalStore

### 关键 actions
- `confirm({ title, description, confirmText, cancelText })` — 返回 Promise<{ ok }>
- `alert({ title, description })`
- `prompt({ title, label, placeholder })` — 返回 Promise<{ ok, value }>
- `handleCancel()` — 当前对话框被中断时

底层用 ui/dialog 的状态，统一队列管理。

---

# 附录 D：UI 组件库的实现细节

## D.1 TabsUnderline.vue
VrcDog 自定义的 Tabs 变体：
- 用下划线高亮 active tab（不是背景色）
- 支持 `unmount-on-hide` 控制卸载策略（影响数据保留/重新加载）
- 支持 `fill` 模式（撑满宽度）

## D.2 DataTableLayout
基于 TanStack Table v8 + Reka UI 的数据表：
- 列定义在 `*Columns.jsx` 文件（用 JSX 因为列里有 render 函数）
- 支持：排序、过滤、分页、虚拟滚动、列调整、行选择
- 底层表组件：`SortableTableHead.vue` 含拖拽手柄

## D.3 VirtualCombobox
虚拟滚动的下拉框，用于：
- 邀请人选择（好友列表 1000+ 时性能必需）
- 群组选择
- 世界选择

## D.4 InputOTP
2FA 验证码输入：
- 6 个独立 slot
- 自动跳焦
- 粘贴时自动填充
- 完成后自动提交（可选）

## D.5 TooltipWrapper
含 side / content / disabled 的统一 tooltip 封装。VrcDog 处处用，复刻时建议先做这个再继续别的。

---

# 附录 E：UserDialog 命令系统（useUserDialogCommands）

VrcDog 把 UserDialog 的所有动作集中在 `useUserDialogCommands.js` composable，返回一个 `userDialogCommand` 对象，子组件通过 `:user-dialog-command="userDialogCommand"` 透传，避免 prop drilling。

### 命令清单（25+ 个）
- 邀请类：`invite()` / `requestInvite()` / `inviteToInstance()` / `dropInvitePortal()`
- 关系类：`addFriend()` / `cancelFriendRequest()` / `unfriend()` / `getFriendStatus()`
- 屏蔽类：`block()` / `unblock()` / `mute()` / `unmute()` / `showAvatar()` / `hideAvatar()`
- 复制类：`copyId()` / `copyUrl()` / `copyDisplayName()`
- 编辑类（自己）：`showSocialStatusDialog()` / `showLanguageDialog()` / `showBioDialog()` / `showPronounsDialog()` / `showEditNoteAndMemoDialog()`
- 头像类（自己）：`showAvatarInfo()` / `showFallbackAvatarInfo()` / `selectFallbackAvatar()`
- 头像类（他人）：`useAvatar()` / `cloneAvatar()`
- 实例类：`refresh()` / `refreshInstance()`
- 备注类：`saveNote()` / `saveMemo()`
- 收藏类：`toggleFavorite()` / `showFavoriteGroupPicker()`
- 邀请响应：`replyInvite()` / `replyRequestInvite()`
- 高级：`sendBoop()` / `previousInstances()` / `compareWithFriend()`

### 注册副作用
还导出 `registerCallbacks(...)` 让父组件注册"打开对话框"的回调（因为子对话框由父组件管理）。

---

# 附录 F：完整文件清单（src/ 下 200+ 文件）

为了完全理解 VrcDog，下面给出**全部源码文件**的扁平清单（不展开测试文件）。

### api/（22 个）
auth, avatar, avatarModeration, favorite, friend, group, image, index, instance, inventory, inviteMessages, misc, notification, playerModeration, prop, queryRequest, user, vrcPlusIcon, vrcPlusImage, world, queryRequest

### components/ 顶层（19 个）
AvatarInfo, BackToTop, CountdownTimer, DeprecationAlert, DisplayName, Emoji, FullscreenImagePreview, InstanceActionBar, Location, LocationWorld, MacOSTitleBar, PresetColorPicker, QuickSearchDialog, QuickSearchSync, StatusBar, statusBarUtils, Timer, UserContextMenu, WorldActionMenuItems

### components/dialogs/ 顶层（13 个）
ChooseFavoriteGroupDialog, CustomNavDialog, DatabaseUpgradeDialog, DialogJsonTab, ImageCropDialog, InviteGroupDialog, LaunchDialog, MainDialogContainer, ModerateGroupDialog, SendBoopDialog, SortableTreeNode, TableLimitsDialog, VrcDogUpdateDialog

### components/dialogs/AvatarDialog/（4）
AvatarDialog, SetAvatarStylesDialog, SetAvatarTagsDialog, useAvatarDialogCommands

### components/dialogs/GroupDialog/（22+）
GallerySelectDialog, GroupDialog, GroupDialogInfoTab, GroupDialogMembersTab, GroupDialogPhotosTab, GroupDialogPostsTab, GroupMemberModerationBanExportDialog, GroupMemberModerationBanImportDialog, GroupMemberModerationDialog, GroupMemberModerationExportDialog, GroupModerationBansTab, GroupModerationBulkActions, GroupModerationInvitesTab, GroupModerationLogsTab, GroupModerationMembersTab, GroupPostEditDialog, useGroupBatchOperations, useGroupCalendarEvents, useGroupDialogCommands, useGroupGalleries, useGroupMembers, useGroupModerationData, useGroupModerationSelection（+ 6 个 *.jsx 列定义）

### components/dialogs/InviteDialog/（5）
EditAndSendInviteDialog, InviteDialog, SendInviteConfirmDialog, SendInviteDialog（+ sendInviteColumns.jsx）

### components/dialogs/NewInstanceDialog/（2）
NewInstanceDialog, useNewInstanceBuilder

### components/dialogs/PreviousInstancesDialog/（5）
PreviousInstancesInfoChart, PreviousInstancesInfoDialog, PreviousInstancesListDialog（+ 2 个 *.jsx 列）

### components/dialogs/UserDialog/（19）
BioDialog, EditNoteAndMemoDialog, LanguageDialog, PronounsDialog, SendInviteRequestDialog, SocialStatusDialog, UserActionDropdown, UserDialog, UserDialogActivityTab, UserDialogAvatarsTab, UserDialogFavoriteWorldsTab, UserDialogGroupCard, UserDialogGroupsTab, UserDialogInfoTab, UserDialogMutualFriendsTab, UserDialogWorldsTab, UserSummaryHeader, useUserDialogCommands, sendInviteRequestColumns.jsx, activity/buildHeatmapOption + DailyPlaytime + （activity/composables）

### components/dialogs/WorldDialog/（7）
SetWorldTagsDialog, useWorldDialogCommands, useWorldDialogInfo, WorldAllowedDomainsDialog, WorldDialog, WorldDialogInfoTab, WorldDialogInstancesTab

### components/ui/（60+ 类，每类 2-15 个文件）
alert, alert-dialog, avatar, badge, breadcrumb, button, button-group, calendar, card, carousel, checkbox, collapsible, command, context-menu, data-table, dialog, dropdown-menu, empty, field, form, hover-card, input, input-group, input-otp, item, kbd, label, native-select, number-field, pagination, popover, progress, radio-group, range-calendar, resizable, scroll-area, select, separator, sheet, sidebar, skeleton, slider, sonner, spinner, switch, table, tabs, tags-input, textarea, toggle, toggle-group, tooltip, tree, virtual-combobox

### composables/（8）
useImageCropper, useInviteChecks, useMainLayoutResizable, useOptionKeySelect, useRecentActions, useToolActions, useToolNavPinning, useUserDisplay

### coordinators/（24）
authAutoLoginCoordinator, authCoordinator, avatarCoordinator, cacheCoordinator, dateCoordinator, favoriteCoordinator, friendPresenceCoordinator, friendRelationshipCoordinator, friendSyncCoordinator, gameCoordinator, gameLogCoordinator, groupCoordinator, imageUploadCoordinator, instanceCoordinator, inviteCoordinator, locationCoordinator, memoCoordinator, moderationCoordinator, searchIndexCoordinator, userCoordinator, userEventCoordinator, userSessionCoordinator, vrcdogCoordinator, worldCoordinator

### services/（11 + database/）
appConfig, config, confusables, gameLog, jsonStorage, request, security, sqlite, watchState, webapi, websocket, database/

### shared/constants/（22）
accessType, api, dashboard, discord, emoji, feedFilters, fonts, group, index, instance, language, link, moderation, photon, remixIconTags.json, settings, tags, themes, tools, ui, user, whatsNewReleases, world

### shared/utils/（35）
_utils, activityEngine, appActions, avatar, avatarTransforms, cacheUtils, chart, common, compare, csv, discordPresence, entityTransforms, fileUtils, friend, gallery, gameLog, group, groupTransforms, imageUpload, index, instance, instanceTransforms, invite, localizationHelperCLI, location, locationParser, notificationCategory, notificationMessage, notificationTransforms, overlapCalculator, platformUtils, quickSearchUtils, resolveRef, retry, setting, throttle, urlUtils, user, userTransforms, world, worldTransforms, base/

### stores/ 顶层（30）
activity, auth, avatar, avatarProvider, charts, dashboard, favorite, feed, friend, gallery, game, group, index, instance, invite, launch, location, modal, moderation, photon, quickSearch, quickSearchWorker, search, searchIndex, sharedFeed, tools, ui, updateLoop, user, vr, vrcStatus, vrcdog, vrcdogUpdater, world

### stores/settings/（6）
advanced, appearance, discordPresence, general, notifications, wristOverlay

### stores/notification/（2）
index, overlayDispatch

### stores/gameLog/（2）
index, mediaParsers

### plugins/（10）
components, dayjs, i18n, index, interopApi, noty, rendererMemoryReport, router, sentry, ui

### queries/（5）
client, entityCache, index, keys, policies, useEntityQueries

### styles/（7 + themes/）
animated-emoji, flags, fonts, globals, noty, options-container, status-icon, themes/

### types/（4 + api/）
common.d.ts, globals.d.ts, index.d.ts, vue-shim.d.ts, api/

### views/（18 个目录）
Charts, Dashboard, Favorites, Feed, FriendList, FriendLog, FriendsLocations, GameLog, Layout, Login, Moderation, MyAvatars, Notifications, PlayerList, Search, Settings, Sidebar, Tools

### vr/
Vr.vue, vr.js, vr.css, components/

### workers/（2）
activityWorker, activityWorkerRunner

### 入口
App.vue, app.js, vite.config.js, index.html, vr.html

---

# 附录 G：复刻执行手册

按本文档复刻时建议遵循的步骤：

1. **先写 i18n** — 把当前阶段需要的 key 写到三个 locale 文件
2. **再写 store 字段** — 在我们的 Pinia store 里加缺失的 ref
3. **写 coordinator** — 把业务逻辑（API 调用 + store 写入 + toast）封装成函数
4. **写组件** — 模板 + 样式 + 把 coordinator 调用绑到事件
5. **跑诊断** — `getDiagnostics` 确认编译通过
6. **手动测试** — 在 dev server 跑起来，对照 VrcDog 截图验收
7. **更新本文档** — 在对应 tab/对话框的 [x] 打勾

每完成一个阶段（A1 到 T），同步更新第 16 节"复刻路线图"和第 17 节"当前进度"。

---

**本文档版本**：4.0（2026-05-17，与真实代码对齐版）
**最近更新**：第 16 节路线图按功能维度重写、第 17 节当前进度按真实代码统计、新增 i18n 缺口与架构层差异说明
**下一轮工作**：W — i18n 补全（10 个空壳语言文件，按 zh-CN 2184 key 集合补真实翻译）→ A8a 编辑器对话框 → A8b 邀请对话框 → 架构层 Z 评估
