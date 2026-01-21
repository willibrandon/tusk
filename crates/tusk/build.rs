//! Build script for Tusk application
//!
//! This script embeds the Windows application manifest for DPI awareness (FR-013).
//! On non-Windows platforms, this script does nothing.

fn main() {
    // Only run on Windows builds
    #[cfg(target_os = "windows")]
    {
        embed_windows_manifest();
    }

    // Re-run if the manifest changes
    println!("cargo:rerun-if-changed=../../assets/tusk.exe.manifest");
}

#[cfg(target_os = "windows")]
fn embed_windows_manifest() {
    use std::env;
    use std::path::PathBuf;

    // Get the manifest path relative to the workspace root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = PathBuf::from(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("assets").join("tusk.exe.manifest"))
        .expect("Could not resolve manifest path");

    if manifest_path.exists() {
        // Use the embed_resource crate if available, otherwise use winres
        // For simplicity, we use the Windows resource compiler directly via println
        println!(
            "cargo:rustc-link-arg-bins=/MANIFEST:EMBED:/MANIFESTINPUT:{}",
            manifest_path.display()
        );
    } else {
        // Fallback: the manifest will be embedded by other means or is not critical for dev builds
        println!(
            "cargo:warning=Windows manifest not found at {:?}, DPI awareness may not be set",
            manifest_path
        );
    }
}
