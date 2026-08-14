# VRCDog / VRCX 功能对照审计（2026-08-14）

本审计基于 VRCX `914ea4d3c4d253a3733d364dbaeff99449c6c202` 的 README 与通知实现，以及 VRChat API community OpenAPI `b7fff1afbf8912def1964bd900f900893cecffd8`。OpenAPI 是社区维护的规格，不是 VRChat 官方承诺；VRChat 可能随时改变未公开接口。

## 结论

- VRCDog 的 VRChat REST 基础层不是假数据层。静态检查到 159 个请求路径，11 个路径没有出现在当前 OpenAPI 路径索引中，但这些路径也被同一版本 VRCX 使用（头像 fallback、文件上传完成、严格群组搜索、反馈、库存等），属于社区客户端实际使用的扩展/未收录接口，不能仅凭 OpenAPI 缺失判定无效。
- 本轮已修复通知链路：旧版通知和 Notification V2 同步、V2 响应数据保留、好友在线/离线/换世界类型一致、分类开关生效、Windows 通知结果可观测、远端清空端点、Pipeline 的 SOCKS5/自定义路径/持续重连。
- “功能入口存在”不代表“功能已达到 VRCX 等价”。下表是当前真实状态，后续开发应按 `partial`/`missing` 处理，而不是继续在 README 中标记为完成。

## 对照矩阵

| VRCX 功能 | VRCDog 状态 | 证据/差距 |
| --- | --- | --- |
| 好友、世界、Avatar、群组列表管理 | partial | 页面和 REST API 存在；部分收藏/群组高级操作没有逐项 UI 闭环验证。 |
| 好友在线、位置、Avatar 实时监控 | implemented | REST 好友同步 + Pipeline 事件 + 本地缓存；私密位置按 VRChat 返回值处理。 |
| 好友历史（添加日期、相处时长、改名） | partial | 有本地 friend log/activity，但没有 VRCX 等价的相处时长和完整改名历史模型。 |
| 多 Dashboard、可配置 Feed/GameLog/Instance widgets | missing | 当前 Dashboard 是固定布局，没有 VRCX 的多个 Dashboard 配置和 widget 过滤持久化。 |
| 用户/世界/Avatar/群组搜索与 Quick Search | partial | SearchView 覆盖主要实体；没有证据证明全部 VRCX fuzzy cache/filter 行为一致。 |
| 活跃热力图 | implemented | 本地 activity 记录 + HeatmapView。数据只从客户端观察开始，不会追溯历史。 |
| 截图中保存世界元数据 | partial | Gallery 可读本地截图；尚未证明会像 VRCX 一样写入/恢复截图对应世界数据。 |
| 通知中心、好友请求、邀请 | implemented (legacy + V2) | 旧版/V2 REST、Pipeline、SQLite、Windows 通知和响应链已对齐；加入请求会按当前实例发送邀请。 |
| 当前实例玩家/统计 | implemented | PlayerListView + gamelog/native bridge。无法从 REST 获取时显示本地日志降级。 |
| 视频播放链接等 GameLog 数据 | partial | 有 gamelog watcher/parser；未覆盖 VRCX 全部日志类型和 UI 字段。 |
| 社交状态预设 | implemented | StatusPresetsView 调用用户状态 API。 |
| VRChat 服务状态 | implemented | 配置状态检查和 UI 状态栏存在。 |
| Discord Rich Presence（人数、平台、缩略图、加入按钮） | partial | 世界名称集成已接通；`discordRpcShowRoomTypeAndCount`、`discordRpcShowPlatform`、`discordRpcShowJoinButton` 等设置目前没有进入 Rust RPC payload。 |
| VR Overlay 实时事件/通知 | partial | 有翻译 Overlay 和 VRPiano Overlay；没有 VRCX 等价的完整事件 Feed Overlay。 |
| 不用 Unity 上传/管理 Avatar/World 图片和详情 | partial | API 模块包含上传流程；需要逐项验证权限错误和 UI 提交闭环。 |
| 启动 VRChat 时自动启动外部应用 | implemented (limited) | systemContext 轮询到 VRChat 启动后调用 `startAutoLaunchApps`；参数校验和失败提示仍较弱。 |
| 崩溃后自动重启并加入上次实例 | missing | 没有保存/恢复上次实例的可靠流程；`autoStart` 已修正为仅控制 VRCDog 的 Windows 开机自启动。 |
| 导入/导出 VRCX 细粒度数据 | partial | VRCDog 支持 SQLite 备份/还原；不是 VRCX 的 friends/avatar/Discord names/favorite groups 选择性 JSON 互导。 |

## 明确的无效/高风险设置

以下设置目前是 UI 存储项，但不能声称已经改变对应运行时行为：

- Discord RPC：房间人数、平台、私密房间显示、加入按钮、世界缩略图等细分开关。
- `autoStartSteamVR`、`vrOverlayEnabled`、`vrOverlayOpacity`、`vrHandTracking`：设置值没有统一的运行时消费者；VR Overlay 启动由具体视图/命令控制。
- `translationApiEnabled`、`youtubeApiEnabled`、`remoteAvatarDbEnabled`、`webApiTimeout`、`requestLimit`：没有统一 API client 消费路径。
- Interface 中的排序、时间格式、随机好友颜色、显示房间 ID 等选项没有在所有相关视图实现。

本轮已删除重复且无消费者的 `showTrayNotifications` 控件；系统通知统一由 Notifications 页的 `notifySystem` 和条件选择器控制。`minimizeToTray` 和 `autoStart` 已分别接通窗口关闭事件和 Windows 注册表命令。

这些入口暂时保留，以免静默丢失用户配置；新增功能时应先补消费者和测试，或者把控件改为明确的“未启用”状态，而不是继续宣称等价 VRCX。

## 验证规则

每个新增 VRChat API 方法必须：

1. 在 `src/api` 中使用真实 `/api/1` 路径和正确 HTTP method/body。
2. 对 401、403、429 分别保留认证失效、权限受限和限流语义。
3. 至少有一个请求形状单测；涉及 Pipeline 的事件必须有重复/重连测试。
4. 页面成功后刷新本地缓存，失败时不得把本地缓存伪装成远端成功。
