use crate::manifest::ResolutionTarget;

use super::{generated_resolution_target, graphics_capability_for_path};

#[test]
fn graphics_abi_paths_map_to_capabilities() {
    assert_eq!(
        graphics_capability_for_path("/usr/lib/libEGL.so.1"),
        Some("graphics.egl")
    );
    assert_eq!(
        graphics_capability_for_path("/usr/lib/libGLESv1_CM.so.1"),
        Some("graphics.gles")
    );
    assert_eq!(
        graphics_capability_for_path("/usr/lib/libGLESv2.so.2"),
        Some("graphics.gles")
    );
    assert_eq!(
        graphics_capability_for_path("/usr/lib/libgbm.so.1"),
        Some("graphics.gbm")
    );
}

#[test]
fn generated_graphics_target_keeps_concrete_fallback() {
    let target = generated_resolution_target("/usr/lib/libEGL.so.1", "mesa".to_string());

    assert_eq!(
        target,
        ResolutionTarget::Capability {
            capability: "graphics.egl".to_string(),
            fallback: Some("mesa".to_string()),
        }
    );
}

#[test]
fn self_and_vulkan_loader_stay_concrete_dependencies() {
    assert_eq!(
        generated_resolution_target("/usr/lib/libEGL.so.1", "self".to_string()),
        ResolutionTarget::Dependency("self".to_string())
    );
    assert_eq!(
        generated_resolution_target("/usr/lib/libvulkan.so.1", "vulkan-loader".to_string()),
        ResolutionTarget::Dependency("vulkan-loader".to_string())
    );
}
