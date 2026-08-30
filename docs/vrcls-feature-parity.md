# VRCLS 功能对照与集成状态

本文记录 VRCDog 对 VRCLS 的功能对照，不把第三方的 WPF/.NET 实现直接搬进 Tauri，而是在现有 Vue + Rust + Python worker 架构中复用同等能力。

## 已落地

| VRCLS 能力 | VRCDog 实现 | 状态 |
| --- | --- | --- |
| 多引擎翻译 | `src-tauri/src/translate.rs`，包含 Google、Google Cloud、Microsoft、DeepL、腾讯、百度、Papago、Gemini、OpenAI-compatible 与本地 LLM | 已接入 |
| 翻译服务档案 | `TranslatorView.vue` 持久化 API、模型、地址、提示词，并支持手动/麦克风/游戏音频/拍照四路独立绑定 | 已接入 |
| 术语库与专名保护 | 请求发送前占位保护、返回后恢复，支持语言范围和 Unicode 安全匹配 | 已接入 |
| 多目标翻译 | 多目标请求并行执行，统一写入 OSC、历史与浮窗 | 已接入 |
| 失败恢复 | 短时缓存、同语言短路、瞬时网络错误指数退避重试 | 已接入 |
| 麦克风与游戏音频 | `audio_capture.rs` + `vrcdog_audio.py`，支持 loopback、指定进程、VAD、降噪、部分结果与本地识别 | 已有并增强 |
| TTS | Web Speech、Edge-TTS bridge、统一服务端 Qwen/MOSS/OmniVoice provider；支持速率、音量、打断、参考音频和参考文本；预设支持保存/切换/导入/导出/删除 | 已接入；云端需配置 endpoint |
| 拍照翻译 | 图片选择或等待下一张 VRChat 截图，Windows OCR 后复用翻译路由、术语库和重试 | 已接入 |
| 桌面/VR 悬浮窗 | `OverlayView.vue` 与 OpenVR OVR 面板，包含玻璃卡片、状态、收起、透明度、OCR 去重 | 已有并增强 |
| OSC Chatbox | 原文/译文模板、typing、Chatbox 长度安全裁剪 | 已有并增强 |

## 部分复用或待补齐

| VRCLS 能力 | 当前情况 | 后续工作 |
| --- | --- | --- |
| Silero VAD | ONNX 模型状态检查、固定 SHA-256 校验、下载命令、运行时切换和实时电平；资源已纳入 Tauri bundle | 已接入；首次构建需运行依赖准备 |
| Sherpa-ONNX 流式识别 | worker 提供 OnlineRecognizer + persistent stream，按 partial/final 解码；四文件路径可配置并在句末 flush | 已接入；需安装 `sherpa-onnx` 和模型 |
| 在线实时 ASR | 腾讯云 v2 签名与阿里 NLS WebSocket 持久会话，支持 partial/final 事件、独立凭据配置和启动前字段验证 | 已接入；需安装 `websocket-client` 和云凭据 |
| TTS 服务档案 | 统一 provider 命令，支持 Edge/Qwen/MOSS/OmniVoice、声音克隆参考音频/文本 | 已接入；音色市场待补 |
| VAD 校准窗口 | 实时 audio level 采样、噪声阶段/语音峰值阶段、建议阈值自动应用 | 已接入 |
| VRChat 照片监听 | 已监听 `Pictures/VRChat` 新文件 | 增加 FileSystemWatcher 原生事件、模式选择和图片结果/文字结果切换 |
| 全局快捷键 | 翻译专用输入/语音快捷键、重复与常用系统键冲突检测、Windows 注册/释放 | 已接入；需桌面模式 |
| 关键词动作 | 识别最终文本匹配关键词，发送 Avatar 参数/任意 OSC，支持类型和值/冷却 | 已接入 |
| 翻译运行时热更新 | 能力清单下载、JSON 校验、可选 SHA-256 校验、本地持久化和版本显示 | 已接入；实际远端清单需发布 |

## 设计边界

- 不复制 VRCLS 的 WPF UI 或 .NET 依赖；所有能力通过现有 Tauri command、Vue 状态和 Python sidecar 落地。
- 图片和音频处理默认只在内存中进行；拍照等待仅读取 VRChat 截图目录，不额外保存处理副本。
- API 密钥继续由本地前端存储和现有安全边界管理，不写入日志或翻译历史。
- `scripts/prepare-python-runtime.mjs` 已加入 `onnxruntime`、`websocket-client` 和 `sherpa-onnx`；执行 `node scripts/prepare-python-runtime.mjs` 后，安装包才会具备完整可选能力。
- Sherpa 需要用户提供匹配的 `tokens.txt`、`encoder.onnx`、`decoder.onnx`、`joiner.onnx`；Silero 模型不能替代 Sherpa 声学模型。
- 腾讯云实时 ASR 需要 AppID/SecretId/SecretKey，阿里云 NLS 需要 AppKey/Token；未配置时保留本地 Whisper/SenseVoice/WebRTC 降级路径。
