//! Tests for sandbox launch construction without Linux namespace support.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;

use super::{environment_variables, launch_script, sandbox_command};
use crate::manifest::BuildEnvironment;

#[test]
fn constructs_the_fixed_unshare_boundary() {
    let variables = BTreeMap::from([("PATH".to_owned(), "/usr/bin".to_owned())]);
    let command = sandbox_command("true", variables);
    let arguments = command
        .get_args()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(command.get_program(), "unshare");
    assert_eq!(
        arguments,
        [
            "--user",
            "--pid",
            "--mount",
            "--uts",
            "--ipc",
            "--net",
            "--fork",
            "--map-root-user",
            "/usr/bin/bash",
            "-eu",
            "-c",
            "true",
        ]
    );
    assert_eq!(command.get_envs().count(), 1);
}

#[test]
fn substitutes_paths_and_cpu_counts_in_launch_data() {
    let root = Path::new("/tmp/build root");
    let script = root.join("nex/tmp/build.sh");
    let environment = environment(true);
    let variables = environment_variables(
        root,
        &environment,
        &BTreeMap::new(),
        NonZeroUsize::new(7).expect("nonzero test CPU count"),
    )
    .expect("construct sandbox environment");
    let launch = launch_script(
        root,
        &script,
        &environment,
        NonZeroUsize::new(7).expect("nonzero test CPU count"),
    )
    .expect("construct chroot launch");

    assert_eq!(variables["ROOT"], "/tmp/build root");
    assert_eq!(variables["MAKEFLAGS"], "-j7");
    assert_eq!(
        launch,
        "cd '/tmp/build root' -j7\nchroot '/tmp/build root' /nex/tmp/build.sh"
    );
}

fn environment(chroot: bool) -> BuildEnvironment {
    serde_yaml::from_str(&format!(
        "name: test\nexecution: {{chroot: {chroot}}}\npaths: {{work: work, out: out, inputs: inputs}}\nenv:\n  ROOT: '{{build_dir}}'\n  MAKEFLAGS: '-j{{num_cpus}}'\npreamble: 'cd {{build_dir}} -j{{num_cpus}}'\n"
    ))
    .expect("test build environment")
}
