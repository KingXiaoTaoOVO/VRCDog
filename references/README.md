# Third-party reference snapshot

This directory contains shallow Git snapshots used for architectural and interoperability research. The projects are **not** part of the VRCDog production build and should not be imported or copied into `src/` or `src-tauri/` without a separate license review.

| Project | Commit | What we reference |
| --- | --- | --- |
| [aps-notecast](https://github.com/Alexs-Piano-Service/aps-notecast) | `782da9c` | MIDI scheduling, tempo scaling, sustain-pedal/control-change routing, recording lifecycle |
| [MioVRC_Translator](https://github.com/CokoIya/MioVRC_Translator) | `611df59` | ASR/TTS fallback strategy, release packaging and resource checks |
| [VRCLS](https://github.com/VoiceLinkVR/VRCLS) | `5b489c4` | Service interfaces, audio pipeline separation, OSC validation, SteamVR panel actions |
| [VRC-Draw](https://github.com/FlyPig01/VRC-Draw) | `efe2b59` | Canonical line-art extraction, topology-safe path optimisation and deterministic execution plans |
| [openvr](https://github.com/ValveSoftware/openvr) | `0924064` | OpenVR API and action/input model |
| [OpenVR-AdvancedSettings](https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings) | `483a010` | Overlay lifecycle, action dispatch and controller-state handling |

Each snapshot keeps its upstream `LICENSE`, `NOTICE` and attribution files. Before shipping any derived code, check the corresponding license and preserve required notices.

## Current VRCDog adoption

- MIDI preview now preserves CC events and extends notes across MIDI sustain-pedal (CC64) cycles.
- The existing Rust piano engine already handles tempo maps, MIDI output, OSC, channel routing, transpose and VR actions; future changes should keep those concerns behind the current `VrpianoApi` boundary.
- The drawing pipeline already exposes a single prepared plan consumed by both preview and execution. Improvements should preserve this invariant and avoid a second preview-only path.
- OpenVR actions should remain data-driven through `src-tauri/vrcdog_actions.json`; do not hard-code controller-specific layouts in Vue components.
