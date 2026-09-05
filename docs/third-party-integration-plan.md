# 第三方能力参考与集成边界

本项目已将指定上游仓库放入 [`references/`](../references/README.md) 做本地源码对照。参考目录不参与 Vite、Cargo 或 Tauri 构建；生产代码只通过现有的 `VrpianoApi`、`DrawingApi`、OSC 和 OpenVR 动作边界接入能力。

## 已完成的第一阶段移植

### 自动弹琴 / MIDI

- `src/audio/generalMidi.ts` 现在保留 MIDI 控制变化事件，并识别通道 64 延音踏板。
- 处于延音状态时，音符释放会延迟到踏板抬起；文件末尾会安全冲刷仍待释放的音符，避免预览截断。
- 既有 Rust 播放器继续负责真实 MIDI 输出、速度、通道静音/独奏、移调、OSC 和 VR 动作；WebAudio 预览与 Rust 播放器共享同一 MIDI 时间语义。
- 该设计借鉴 aps-notecast 的“事件序列 + pedal routing”思路，但没有复制其 Android 服务代码或第三方依赖。

### 自动绘画 / 识别

- 当前 Rust 管线已经具备规范化、平滑、Douglas–Peucker 简化、路径合并、2-opt 顺序优化、暂停/恢复和确定性执行计划。
- 继续沿用 VRC-Draw 的关键不变量：预览和真实鼠标执行必须消费同一个 `PreparedDrawing`，不能维护两套“看起来更平滑”的路线。
- 后续可在不改变公开配置的前提下增加局部曲线拟合，但必须保留逐跨度回退到折线的安全门禁。

### VR 操作

- OpenVR 控制器动作继续由 `src-tauri/vrcdog_actions.json` 数据驱动，并由 Rust overlay 状态机派发到前端事件。
- Advanced Settings 的经验表明，动作读取、overlay 生命周期和输入优先级应分离；新增 VR 功能不得在 Vue 中直接绑定控制器型号。
- VRCLS 的音频/OSC 分层可用于后续把“音频捕获 → VAD → ASR → 翻译 → TTS → Chatbox/Overlay”拆成可观测的服务阶段。

## 第二阶段建议

1. 在 `MidiParseResult` 上增加可选的 tempo map/拍号元数据，为节拍器、跟拍和片段循环提供统一时间基准。
2. 为 MIDI 播放增加“踏板可视化”和控制器事件过滤设置，默认保持现有行为。
3. 给绘画执行计划增加每条笔画的边界框、预计耗时和失败回退原因，便于 VR 内调试。
4. 把 VR overlay 动作状态统一成 `idle / armed / active / unavailable`，并为 SteamVR 未运行提供优雅降级提示。
5. 对 ASR/TTS、OSC、OpenVR 和 MIDI 设备增加资源释放检查，确保窗口关闭、设备拔出和 SteamVR 重启时不会残留线程或句柄。

## 许可证与发布

第三方仓库各自的 `LICENSE`/`NOTICE` 文件保留在对应快照目录。任何未来的代码级移植都必须先完成许可证核对、依赖隔离、版权声明和可复现构建检查；本阶段没有把第三方源码直接合并进生产代码。
