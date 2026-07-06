#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]
// Cross-build-friendly replacement for zed's `media` crate build.rs.
//
// The upstream build.rs is gated on `#[cfg(target_os = "macos")]`, which is the
// HOST os while the build script compiles — so cross-building from Linux runs
// the empty branch and never generates bindings.rs, breaking the target-side
// `include!`. It also shells out to `xcrun` (Apple-only).
//
// This version keys off the TARGET (CARGO_CFG_TARGET_OS) and resolves the SDK
// from SDKROOT, falling back to `xcrun` when building natively on a Mac.
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "macos" && target_os != "ios" {
        return;
    }

    use std::{env, path::PathBuf, process::Command};

    let sdk_path = env::var("SDKROOT").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| {
        String::from_utf8(
            Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .expect("SDKROOT unset and `xcrun` not available")
                .stdout,
        )
        .unwrap()
        .trim_end()
        .to_string()
    });

    let target = env::var("TARGET").unwrap_or_default();

    println!("cargo:rerun-if-changed=src/bindings.h");
    println!("cargo:rerun-if-env-changed=SDKROOT");

    let bindings = bindgen::Builder::default()
        .header("src/bindings.h")
        .clang_arg(format!("-isysroot{}", sdk_path))
        .clang_arg(format!("-I{}/usr/include", sdk_path))
        .clang_arg(format!("-F{}/System/Library/Frameworks", sdk_path))
        .clang_arg(format!("--target={}", target))
        .clang_arg("-xobjective-c")
        .allowlist_type("CMItemIndex")
        .allowlist_type("CMSampleTimingInfo")
        .allowlist_type("CMVideoCodecType")
        .allowlist_type("VTEncodeInfoFlags")
        .allowlist_function("CMTimeMake")
        .allowlist_var("kCVPixelFormatType_.*")
        .allowlist_var("kCVReturn.*")
        .allowlist_var("VTEncodeInfoFlags_.*")
        .allowlist_var("kCMVideoCodecType_.*")
        .allowlist_var("kCMTime.*")
        .allowlist_var("kCMSampleAttachmentKey_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("couldn't write dispatch bindings");
}
