fn main() {
    let ctx = unsafe { openvr::init(openvr::ApplicationType::Overlay).unwrap() };
    let chap = ctx.chaperone_setup().unwrap();
    let mut mat = openvr::pose::Matrix3x4([[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]]);
    chap.set_working_standing_zero_pose_to_raw_tracking_pose(&mat);
    chap.commit_working_copy(openvr::ConfigFile::Live);
}
