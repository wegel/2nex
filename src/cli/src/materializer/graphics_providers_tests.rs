use super::{is_graphics_provider_path, is_nvidia_provider_commit, uses_graphics_loader};

#[test]
fn detects_nvidia_provider_commits() {
    assert!(is_nvidia_provider_commit(
        "x86_64/pkg/libs/graphics/nvidia-580/580.159.04/bundles/runtime"
    ));
    assert!(is_nvidia_provider_commit(
        "x86_64/pkg/libs/graphics/nvidia-current/595.84/bundles/runtime"
    ));
    assert!(!is_nvidia_provider_commit(
        "x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/lib"
    ));
}

#[test]
fn selects_loader_provider_files() {
    assert!(is_graphics_provider_path(
        "/usr/share/glvnd/egl_vendor.d/10_nvidia.json"
    ));
    assert!(is_graphics_provider_path(
        "/usr/share/egl/egl_external_platform.d/15_nvidia_gbm.json"
    ));
    assert!(is_graphics_provider_path("/usr/lib/gbm/nvidia-drm_gbm.so"));
    assert!(is_graphics_provider_path("/usr/lib/libEGL.so.1"));
    assert!(is_graphics_provider_path("/usr/lib/libEGL_nvidia.so.0"));
    assert!(is_graphics_provider_path("/usr/lib/libGLESv2.so.2"));
    assert!(is_graphics_provider_path("/usr/lib/libGLdispatch.so.0"));
    assert!(is_graphics_provider_path(
        "/usr/lib/libnvidia-allocator.so.1"
    ));
    assert!(!is_graphics_provider_path("/usr/bin/nvidia-smi"));
}

#[test]
fn detects_graphics_loader_capsules() {
    let temp_dir = tempfile::TempDir::new().expect("test setup should succeed");
    let lib_dir = temp_dir.path().join("usr/lib");
    std::fs::create_dir_all(&lib_dir).expect("test setup should succeed");
    std::fs::write(lib_dir.join("libEGL.so.1"), b"").expect("test setup should succeed");

    assert!(uses_graphics_loader(temp_dir.path()));
}
