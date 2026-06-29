use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::manifest::types::{BuildEnvironment, BuildPaths, ExecutionConfig};

use super::{build_script_env, BuildLaunch};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn host_build_environment() -> BuildEnvironment {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());

    BuildEnvironment {
        name: "host-test".to_string(),
        description: String::new(),
        execution: ExecutionConfig { chroot: false },
        env,
        preamble: String::new(),
        paths: BuildPaths {
            work: "nex/work".to_string(),
            out: "nex/out".to_string(),
            inputs: "inputs".to_string(),
        },
    }
}

fn test_launch() -> BuildLaunch {
    BuildLaunch {
        build_dir_str: "/tmp/nex-build-test".to_string(),
        tmpdir_path: PathBuf::from("/tmp/nex-build-test/nex/tmp"),
        num_cpus: 1,
        bootstrap_tools: "/tmp/nex-build-test/bootstrap/tools".to_string(),
        bootstrap_sysroot: "/tmp/nex-build-test/bootstrap".to_string(),
    }
}

#[test]
fn host_build_env_does_not_clear_parent_process_env() {
    let _guard = env_lock().lock().unwrap();
    let key = "NEX_TEST_PARENT_ENV_SURVIVES";
    let previous = env::var_os(key);
    env::set_var(key, "kept");

    let script_env = build_script_env(&test_launch(), &HashMap::new(), &host_build_environment());

    assert_eq!(env::var(key).as_deref(), Ok("kept"));
    assert!(!script_env.contains_key(key));

    match previous {
        Some(value) => env::set_var(key, value),
        None => env::remove_var(key),
    }
}
