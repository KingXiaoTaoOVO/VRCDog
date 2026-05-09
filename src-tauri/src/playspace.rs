use openvr_sys as sys;

pub struct PlayspaceController {
    setup_ptr: *mut sys::VR_IVRChaperoneSetup_FnTable,
    base_pose: sys::HmdMatrix34_t,
    original_pose: sys::HmdMatrix34_t,
}

// SAFETY: The OpenVR function table pointer is static and valid for the lifetime of the application.
// Struct contains only data and a read-only function table pointer, making it safe to share across threads.
unsafe impl Send for PlayspaceController {}
unsafe impl Sync for PlayspaceController {}

impl PlayspaceController {
    /// Initializes the PlayspaceController, returning a Result with the OpenVR error code if it fails.
    pub fn new() -> Result<Self, sys::EVRInitError> {
        let mut magic = Vec::from(b"FnTable:" as &[u8]);
        magic.extend_from_slice(sys::IVRChaperoneSetup_Version);
        let mut err = sys::EVRInitError_VRInitError_None;
        
        let ptr = unsafe { sys::VR_GetGenericInterface(magic.as_ptr() as *const i8, &mut err) };
        if ptr != 0 {
            let setup_ptr = ptr as *mut sys::VR_IVRChaperoneSetup_FnTable;
            let mut base_pose = sys::HmdMatrix34_t { m: [[0.0; 4]; 3] };
            
            // Get current pose as base
            unsafe {
                if let Some(get_fn) = (*setup_ptr).GetWorkingStandingZeroPoseToRawTrackingPose {
                    get_fn(&mut base_pose);
                }
            }

            Ok(Self {
                setup_ptr,
                base_pose,
                original_pose: base_pose,
            })
        } else {
            Err(err)
        }
    }

    pub fn apply_offset(&self, dx: f32, dy: f32, dz: f32, rot_deg: f32) {
        let mut new_pose = self.base_pose;
        
        // 1. Rotation around Y axis (yaw)
        let rad = rot_deg.to_radians();
        let cos_r = rad.cos();
        let sin_r = rad.sin();
        
        // For rotation, we multiply the base rotation matrix by a Y-rotation matrix.
        let r00 = self.base_pose.m[0][0]; let r01 = self.base_pose.m[0][1]; let r02 = self.base_pose.m[0][2];
        let r20 = self.base_pose.m[2][0]; let r21 = self.base_pose.m[2][1]; let r22 = self.base_pose.m[2][2];
        
        // Apply Y rotation
        new_pose.m[0][0] = r00 * cos_r + r20 * sin_r;
        new_pose.m[0][1] = r01 * cos_r + r21 * sin_r;
        new_pose.m[0][2] = r02 * cos_r + r22 * sin_r;
        
        new_pose.m[2][0] = -r00 * sin_r + r20 * cos_r;
        new_pose.m[2][1] = -r01 * sin_r + r21 * cos_r;
        new_pose.m[2][2] = -r02 * sin_r + r22 * cos_r;

        // 2. Translation
        let rot_dx = dx * cos_r + dz * sin_r;
        let rot_dz = -dx * sin_r + dz * cos_r;

        new_pose.m[0][3] = self.base_pose.m[0][3] - rot_dx;
        new_pose.m[1][3] = self.base_pose.m[1][3] - dy;
        new_pose.m[2][3] = self.base_pose.m[2][3] - rot_dz;

        // 3. Set and Commit
        unsafe {
            if let Some(set_fn) = (*self.setup_ptr).SetWorkingStandingZeroPoseToRawTrackingPose {
                set_fn(&mut new_pose);
            }
            if let Some(commit_fn) = (*self.setup_ptr).CommitWorkingCopy {
                commit_fn(sys::EChaperoneConfigFile_Live);
            }
        }
    }

    pub fn set_base_floor_to(&mut self, controller_y_raw: f32) {
        // Sets the floor by modifying the base_pose Y translation directly
        self.base_pose.m[1][3] -= controller_y_raw;
        
        // Immediately apply
        unsafe {
            if let Some(set_fn) = (*self.setup_ptr).SetWorkingStandingZeroPoseToRawTrackingPose {
                set_fn(&mut self.base_pose);
            }
            if let Some(commit_fn) = (*self.setup_ptr).CommitWorkingCopy {
                commit_fn(sys::EChaperoneConfigFile_Live);
            }
        }
    }

    pub fn reset(&mut self) {
        // Since we commit changes to Live, RevertWorkingCopy won't restore the initial state.
        // We must re-apply the original pose we saved at startup.
        self.base_pose = self.original_pose;
        
        unsafe {
            if let Some(set_fn) = (*self.setup_ptr).SetWorkingStandingZeroPoseToRawTrackingPose {
                set_fn(&mut self.original_pose);
            }
            if let Some(commit_fn) = (*self.setup_ptr).CommitWorkingCopy {
                commit_fn(sys::EChaperoneConfigFile_Live);
            }
        }
    }
}
