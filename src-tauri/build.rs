#[cfg(target_os = "macos")]
fn build_macos_asr_bridge() {
    use std::env;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let bridge = manifest_dir.join("native/asr_bridge.m");

    println!("cargo:rerun-if-changed={}", bridge.display());

    cc::Build::new()
        .file(bridge)
        .flag("-fobjc-arc")
        .compile("asr_bridge");

    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Speech");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
}

fn main() {
    #[cfg(target_os = "macos")]
    build_macos_asr_bridge();

    tauri_build::build()
}
