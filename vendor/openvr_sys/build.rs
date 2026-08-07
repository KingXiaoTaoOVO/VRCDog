#[cfg(feature = "buildtime_bindgen")]
extern crate bindgen;
extern crate cmake;

use std::env;

fn main() {
    let mut config = cmake::Config::new("openvr");
    config.define("CMAKE_POLICY_VERSION_MINIMUM", "3.5");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();

    if target_os == "macos" {
        config.define("BUILD_UNIVERSAL", "OFF");
    } else if target_os == "windows" {
        // Build OpenVR without Debug CRT references so Rust test binaries can link it.
        config.define("CMAKE_BUILD_TYPE", "Release");
        config.define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreadedDLL");
        // Work around broken cmake build.
        config.cxxflag("/DWIN32");
        // Force the Visual Studio *multi-config* generator to build the Release
        // configuration. For multi-config generators CMAKE_BUILD_TYPE is ignored,
        // and the `cmake` crate otherwise passes `--config Debug` on `tauri dev`
        // (cargo debug profile), producing an openvr_api64.lib linked against the
        // debug CRT (/MDd). That conflicts with the app's release CRT and fails the
        // final link with LNK4098 / unresolved __imp__CrtDbgReport / LNK1120.
        config.profile("Release");
    }

    let dst = config.build();
    println!("cargo:rustc-link-search=native={}/lib", dst.display());

    if target_os == "windows" && target_pointer_width == "64" {
        println!("cargo:rustc-link-lib=static=openvr_api64");
    } else {
        println!("cargo:rustc-link-lib=static=openvr_api");
    }

    if target_os == "linux" {
        println!("cargo:rustc-link-lib=stdc++");
    } else if target_os == "macos" {
        println!("cargo:rustc-link-lib=c++");
    } else if target_os == "windows" {
        println!("cargo:rustc-link-lib=shell32");
    }

    // Generate bindings at build time.
    #[cfg(feature = "buildtime_bindgen")]
    bindgen::builder()
        .header("wrapper.hpp")
        .constified_enum(".*")
        .prepend_enum_name(false)
        .generate()
        .expect("could not generate bindings")
        .write_to_file("bindings.rs")
        .expect("could not write bindings.rs");
}
