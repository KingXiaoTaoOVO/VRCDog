#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

include!("bindings.rs");

// Compatibility shims for the `openvr` 0.9 wrapper. This vendored crate uses
// pre-generated bindings so Windows builds do not require libclang/bindgen.
pub const ETrackedDeviceProperty_Prop_PreviousUniverseId_Uint64_deprecated:
    ETrackedDeviceProperty = 0;
pub const EVREventType_VREvent_ChaperoneRoomSetupCommitted: EVREventType = 807;
pub const EVRInitError_VRInitError_Compositor_CreateBlurTexture: EVRInitError = 500;
pub const EVRSettingsError_VRSettingsError_AccessDenied: EVRSettingsError = 6;

impl Default for TrackedDevicePose_t {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Default for Compositor_FrameTiming {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(target_os = "macos")]
#[link(name = "Foundation", kind = "framework")]
extern "C" {}
