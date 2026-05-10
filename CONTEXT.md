# 架构上下文词汇表 (Context & Domain Glossary)

## 核心领域概念 (Core Domain Concepts)

- **VrcDog Client (客户端)**: 基于 Tauri + Vue3 + Rust 开发的轻量级 VRChat 辅助工具集。目标是提供类似 VRCX 的体验，但在性能和 UI 上遵循极致美学 (`leizhi-factory` 规范)。
- **UserProfile (用户详情)**: 跨系统通用的全局用户展示组件。负责聚合来自 VRChat 官方 API 的线上数据（如：基础信息、群组、世界）与本地 SQLite `DbApi` 维护的离线数据（如：活动记录、本地备注）。
- **Hybrid Async Loading (混合异步加载)**: 用户详情面板的核心加载策略。即打开面板时瞬间展现骨架与基础信息，各个复杂 Tab（共同好友、活动等）按需懒加载并自持有 Loading 状态。
- **Store-driven Modal (状态驱动的模态框)**: 弃用多实例弹窗，改由 `useUserProfileStore` 统管的单例全局组件，通过动态替换 `targetUserId` 来复用同一个 DOM 和请求通道。
- **Graceful Fallback (优雅降级)**: 在处理如“共同好友”等可能涉及 VRChat 隐私限制的数据时，若遇到 401 或受限情况，不进行无意义的本地暴力猜测，而是向用户明确展示“对方隐藏了此信息”的优雅提示。
- **Inline Aggregation & Auto-save (内联聚合与无缝保存)**: 本地离线数据（如备注）与线上数据在同一层级聚合展示。修改本地备注采用内联文本框结合失去焦点(OnBlur)静默保存的设计，避免二次弹窗打断操作流。
- **Context-Aware Triggers (环境感知触发器)**: 细粒度的通知与 TTS 播放控制。底层 Rust 持续轮询 SteamVR 与 VRChat 的进程状态，前端依据当前应用所处的上下文（桌面、VR、离线等）动态决断是否放行通知。

## 架构决策记录 (ADRs)

详细请参见 `docs/adr/`。
