//! Environment variable assembly for system build scripts.

use std::collections::HashMap;
use std::io;

use crate::build::handle_inputs;
use crate::manifest::{BuildPaths, SystemManifest};

/// Build environment variables passed to a system assembly script.
pub fn build_system_env_vars(
    manifest: &SystemManifest,
    download_dir: &str,
    base_dir: &str,
    use_absolute_paths: bool,
    paths: &BuildPaths,
) -> io::Result<HashMap<String, String>> {
    let mut env_vars = handle_inputs(
        &manifest.sources,
        download_dir,
        base_dir,
        use_absolute_paths,
        paths,
        None,
    )?;
    env_vars.insert("SYSTEM_NAME".to_string(), manifest.system.name.clone());
    env_vars.insert("SYSTEM_SLUG".to_string(), manifest.system.slug.clone());
    env_vars.insert(
        "SYSTEM_VERSION".to_string(),
        manifest.system.version.clone(),
    );
    env_vars.insert("TARGET_DIR".to_string(), "/target".to_string());
    env_vars.insert("SYSTEM_TARGET".to_string(), "/target".to_string());
    insert_optional_system_vars(manifest, &mut env_vars);
    Ok(env_vars)
}

fn insert_optional_system_vars(manifest: &SystemManifest, env_vars: &mut HashMap<String, String>) {
    if let Some(arch) = &manifest.system.architecture {
        env_vars.insert("SYSTEM_ARCH".to_string(), arch.clone());
    }
    if let Some(boot) = &manifest.system.boot_method {
        env_vars.insert("SYSTEM_BOOT_METHOD".to_string(), boot.clone());
    }
    if let Some(desc) = &manifest.system.description {
        env_vars.insert("SYSTEM_DESCRIPTION".to_string(), desc.clone());
    }
}
