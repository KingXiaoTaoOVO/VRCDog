#[cfg(feature = "buildtime_bindgen")]
extern crate bindgen;
extern crate cmake;

use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();

    let manifest_dir = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let prebuilt_dir = manifest_dir.join("openvr").join("bin").join("win64");
    if target_os == "windows" && target_pointer_width == "64" && prebuilt_dir.join("openvr_api64.lib").exists() {
        println!("cargo:rustc-link-search=native={}", prebuilt_dir.display());
        println!("cargo:rustc-link-lib=static=openvr_api64");
        println!("cargo:rustc-link-lib=shell32");
        return;
    }

    let mut config = cmake::Config::new("openvr");
    config.define("CMAKE_POLICY_VERSION_MINIMUM", "3.5");

    if target_os == "macos" {
        config.define("BUILD_UNIVERSAL", "OFF");
    } else if target_os == "windows" {
        // Build OpenVR without Debug CRT references so Rust test binaries can link it.
        config.define("CMAKE_BUILD_TYPE", "Release");
        config.define("CMAKE_MSVC_RUNTIME_LIBRARY", "MultiThreadedDLL");
        // Work around broken cmake build.
        config.cxxflag("/DWIN32");
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
