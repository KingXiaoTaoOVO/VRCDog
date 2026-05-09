import sys
import re

lib_path = r"c:\Users\27457\Desktop\Project\UnityEXE\src-tauri\src\lib.rs"
with open(lib_path, "r", encoding="utf-8") as f:
    lib_content = f.read()

if "pub mod playspace;" not in lib_content:
    lib_content = lib_content.replace("pub mod toolchain;", "pub mod toolchain;\npub mod playspace;")
    with open(lib_path, "w", encoding="utf-8") as f:
        f.write(lib_content)

ovr_path = r"c:\Users\27457\Desktop\Project\UnityEXE\src-tauri\src\ovr.rs"
with open(ovr_path, "r", encoding="utf-8") as f:
    ovr_content = f.read()

# 1. Add initialization of PlayspaceController
init_target = "    let mut last_ocr_text = String::new();"
init_new = """    let mut last_ocr_text = String::new();
    let mut playspace = crate::playspace::PlayspaceController::new();"""
ovr_content = ovr_content.replace(init_target, init_new)

# 2. Update OvrCommand::SetPlayspaceOffset
offset_target = """                OvrCommand::SetPlayspaceOffset { x, y, z } => {
                    ps_offset_x = x;
                    ps_offset_y = y;
                    ps_offset_z = z;
                    // Apply offset: We modify the overlay's universe position
                    // For a true OVRAS-like experience, we use chaperone setup if available,
                    // otherwise we apply the offset to all active overlays.
                    if let Ok(_sys) = context.system() {
                        let _ = app_handle.emit("ovr_log", "[OVR] ⚠ openvr crate 暂不支持 ChaperoneSetup API，原生空间偏移无法生效");
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }"""

offset_new = """                OvrCommand::SetPlayspaceOffset { x, y, z } => {
                    ps_offset_x = x;
                    ps_offset_y = y;
                    ps_offset_z = z;
                    if let Some(ref mut ps) = playspace {
                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }"""
ovr_content = ovr_content.replace(offset_target, offset_new)

# 3. Update SetPlayspaceRotation
rot_target = """                OvrCommand::SetPlayspaceRotation(deg) => {
                    ps_rotation_deg = deg;
                    let _ = app_handle.emit("ovr_log", "[OVR] ⚠ openvr crate 暂不支持 ChaperoneSetup API，空间旋转无法生效");
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }"""

rot_new = """                OvrCommand::SetPlayspaceRotation(deg) => {
                    ps_rotation_deg = deg;
                    if let Some(ref mut ps) = playspace {
                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }"""
ovr_content = ovr_content.replace(rot_target, rot_new)

# 4. Update ToggleHeight
height_target = """                OvrCommand::ToggleHeight => {
                    height_toggled = !height_toggled;
                    let _y_change = if height_toggled { -height_offset } else { height_offset };
                    let _ = app_handle.emit("ovr_log", "[OVR] ⚠ openvr crate 暂不支持 ChaperoneSetup API，高度切换无法生效");
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }"""

height_new = """                OvrCommand::ToggleHeight => {
                    height_toggled = !height_toggled;
                    if let Some(ref mut ps) = playspace {
                        let y_total = ps_offset_y + if height_toggled { height_offset } else { 0.0 };
                        ps.apply_offset(ps_offset_x, y_total, ps_offset_z, ps_rotation_deg);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": ps_offset_x, "offset_y": ps_offset_y, "offset_z": ps_offset_z,
                        "rotation": ps_rotation_deg, "height_toggled": height_toggled,
                    }));
                }"""
ovr_content = ovr_content.replace(height_target, height_new)

# 5. Update ResetPlayspace
reset_target = """                OvrCommand::ResetPlayspace => {
                    ps_offset_x = 0.0;
                    ps_offset_y = 0.0;
                    ps_offset_z = 0.0;
                    ps_rotation_deg = 0.0;
                    height_toggled = false;
                    let _ = app_handle.emit("ovr_log", "[OVR] ⚠ openvr crate 暂不支持 ChaperoneSetup API，重置空间无法生效");
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                        "rotation": 0.0, "height_toggled": false,
                    }));
                }"""

reset_new = """                OvrCommand::ResetPlayspace => {
                    ps_offset_x = 0.0;
                    ps_offset_y = 0.0;
                    ps_offset_z = 0.0;
                    ps_rotation_deg = 0.0;
                    height_toggled = false;
                    if let Some(ref mut ps) = playspace {
                        ps.apply_offset(0.0, 0.0, 0.0, 0.0);
                    }
                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                        "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                        "rotation": 0.0, "height_toggled": false,
                    }));
                }"""
ovr_content = ovr_content.replace(reset_target, reset_new)

# 6. Update FixFloor
fix_target = """                OvrCommand::FixFloor => {
                    // Fix floor by placing right controller on the floor
                    // We read the controller's Y position and use it as the floor offset
                    if let Ok(sys) = context.system() {
                        if let Some(r_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand) {
                            let poses = sys.device_to_absolute_tracking_pose(openvr::TrackingUniverseOrigin::Standing, 0.0);
                            let pose = poses[r_idx.0 as usize];
                            if pose.pose_is_valid() {
                                let mat = pose.device_to_absolute_tracking();
                                let _controller_y = mat[1][3]; // Height of controller
                                // The controller on the floor should be at Y≈0
                                // So we offset by -controller_y to bring floor to 0
                                let _ = app_handle.emit("ovr_log", "[OVR] ⚠ openvr crate 暂不支持 ChaperoneSetup API，修复地板无法生效");
                            } else {
                                let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 右手柄未跟踪，无法修复地板");
                            }
                        } else {
                            let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 未检测到右手柄");
                        }
                    }
                }"""

fix_new = """                OvrCommand::FixFloor => {
                    if let Ok(sys) = context.system() {
                        if let Some(r_idx) = sys.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand) {
                            let poses = sys.device_to_absolute_tracking_pose(openvr::TrackingUniverseOrigin::Standing, 0.0);
                            let pose = poses[r_idx.0 as usize];
                            if pose.pose_is_valid() {
                                let mat = pose.device_to_absolute_tracking();
                                let controller_y = mat[1][3];
                                if let Some(ref mut ps) = playspace {
                                    ps.set_base_floor_to(controller_y);
                                    // Also clear local UI offset tracking since base is changed
                                    ps_offset_x = 0.0;
                                    ps_offset_y = 0.0;
                                    ps_offset_z = 0.0;
                                    ps_rotation_deg = 0.0;
                                    height_toggled = false;
                                    let _ = app_handle.emit("ovr_log", "[OVR] ✅ 地板已修复！(右手柄位置置底)");
                                    let _ = app_handle.emit("ovr_playspace_changed", serde_json::json!({
                                        "offset_x": 0.0, "offset_y": 0.0, "offset_z": 0.0,
                                        "rotation": 0.0, "height_toggled": false,
                                    }));
                                }
                            } else {
                                let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 右手柄未跟踪，无法修复地板");
                            }
                        } else {
                            let _ = app_handle.emit("ovr_log", "[OVR] ⚠ 未检测到右手柄");
                        }
                    }
                }"""
ovr_content = ovr_content.replace(fix_target, fix_new)

with open(ovr_path, "w", encoding="utf-8") as f:
    f.write(ovr_content)
print("Updated lib.rs and ovr.rs for Native Playspace Controller.")
