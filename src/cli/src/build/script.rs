//! Build script execution under the selected build environment.

use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::manifest::types::BuildEnvironment;
use crate::progress::{self, BuildProgressConfig};

use super::env::expand_env_templates;
use super::rootfs::{refresh_chroot_usrmerge_symlinks, validate_chroot_build_root};

/// Result from running a build script with optional profile recording.
pub struct BuildScriptResult {
    /// New build profile data when `--record-profile` was enabled.
    pub new_profile: Option<Vec<String>>,
}

struct BuildLaunch {
    build_dir_str: String,
    tmpdir_path: PathBuf,
    num_cpus: usize,
    bootstrap_tools: String,
    bootstrap_sysroot: String,
}

struct ProgressCapture {
    new_profile: Option<Vec<String>>,
    output: Option<Vec<u8>>,
}

/// Run a build script in the namespace and chroot mode named by its environment.
pub fn run_build_script_with_env(
    build_script: &str,
    build_dir: &str,
    input_env_vars: &HashMap<String, String>,
    build_env: &BuildEnvironment,
    progress_config: Option<&BuildProgressConfig>,
) -> io::Result<BuildScriptResult> {
    println!(
        "Running build script in isolated environment using '{}' environment",
        build_env.name
    );

    let launch = prepare_launch(build_dir, build_env)?;
    let script_env = build_script_env(&launch, input_env_vars, build_env);
    let launch_script = build_launch_script(build_script, build_env, &launch)?;
    let mut child = spawn_build_process(&script_env, &launch_script, progress_config)?;
    let capture = capture_progress(&mut child, progress_config)?;

    finish_build_process(child, capture)
}

fn prepare_launch(build_dir: &str, build_env: &BuildEnvironment) -> io::Result<BuildLaunch> {
    let build_dir = absolute_build_dir(build_dir)?;
    let build_dir_str = build_dir
        .to_str()
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "build_dir must be valid UTF-8")
        })?
        .to_string();
    let tmpdir_path = build_dir.join("nex").join("tmp");
    std::fs::create_dir_all(&tmpdir_path)?;

    if build_env.execution.chroot {
        refresh_chroot_usrmerge_symlinks(&build_dir)?;
        validate_chroot_build_root(&build_dir)?;
    }

    let bootstrap_sysroot = build_dir.join("bootstrap");
    let bootstrap_tools = bootstrap_sysroot.join("tools");
    Ok(BuildLaunch {
        build_dir_str,
        tmpdir_path,
        num_cpus: num_cpus::get(),
        bootstrap_tools: path_to_string(&bootstrap_tools)?,
        bootstrap_sysroot: path_to_string(&bootstrap_sysroot)?,
    })
}

fn absolute_build_dir(build_dir: &str) -> io::Result<PathBuf> {
    let path = Path::new(build_dir);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn path_to_string(path: &Path) -> io::Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path must be valid UTF-8"))
}

fn build_script_env(
    launch: &BuildLaunch,
    input_env_vars: &HashMap<String, String>,
    build_env: &BuildEnvironment,
) -> HashMap<String, String> {
    if !build_env.execution.chroot {
        for (key, _) in env::vars() {
            env::remove_var(key);
        }
    }

    let mut vars = HashMap::new();
    for (key, value) in &build_env.env {
        vars.insert(key.clone(), expand_for_launch(value, launch));
    }
    for (key, value) in input_env_vars {
        vars.insert(key.clone(), value.clone());
    }
    vars
}

fn build_launch_script(
    build_script: &str,
    build_env: &BuildEnvironment,
    launch: &BuildLaunch,
) -> io::Result<String> {
    if build_env.execution.chroot {
        write_chroot_script(build_script, build_env, launch)
    } else {
        Ok(write_host_script(build_script, build_env, launch))
    }
}

fn write_chroot_script(
    build_script: &str,
    build_env: &BuildEnvironment,
    launch: &BuildLaunch,
) -> io::Result<String> {
    let temp_file_path = launch.tmpdir_path.join("build_script.sh");
    let mut temp_file = std::fs::File::create(&temp_file_path)?;
    temp_file.write_all(b"#!/usr/bin/bash -eu\n")?;
    temp_file.write_all(build_script.as_bytes())?;

    let preamble = expand_for_launch(&build_env.preamble, launch);
    Ok(format!(
        r#"
{preamble}

chmod +x {build_dir}/nex/tmp/build_script.sh
unshare --root={build_dir} /nex/tmp/build_script.sh 2>&1
"#,
        preamble = preamble,
        build_dir = launch.build_dir_str
    ))
}

fn write_host_script(
    build_script: &str,
    build_env: &BuildEnvironment,
    launch: &BuildLaunch,
) -> String {
    let preamble = expand_for_launch(&build_env.preamble, launch);
    if preamble.trim().is_empty() {
        build_script.to_string()
    } else {
        format!("{preamble}\n{build_script}")
    }
}

fn spawn_build_process(
    script_env: &HashMap<String, String>,
    launch_script: &str,
    progress_config: Option<&BuildProgressConfig>,
) -> io::Result<Child> {
    let mut command = Command::new("unshare");
    command
        .args(unshare_command(launch_script))
        .envs(script_env);

    if progress_config.is_some() {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
    }

    println!(
        "Executing build script under unshare with env vars: {:?}",
        script_env
    );
    command.spawn()
}

fn capture_progress(
    child: &mut Child,
    progress_config: Option<&BuildProgressConfig>,
) -> io::Result<ProgressCapture> {
    let Some(config) = progress_config else {
        return Ok(ProgressCapture {
            new_profile: None,
            output: None,
        });
    };

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("failed to capture stdout"))?;
    let stderr_handle = child.stderr.take().map(read_stream_in_background);
    let result = progress::run_with_progress(stdout, config)?;
    let mut output = result.captured_output.unwrap_or_default();

    if let Some(stderr) = stderr_handle.and_then(|handle| handle.join().ok()) {
        output.extend(stderr);
    }

    Ok(ProgressCapture {
        new_profile: result.new_profile,
        output: (!output.is_empty()).then_some(output),
    })
}

fn finish_build_process(
    mut child: Child,
    capture: ProgressCapture,
) -> io::Result<BuildScriptResult> {
    let result = child.wait()?;
    if result.success() {
        return Ok(BuildScriptResult {
            new_profile: capture.new_profile,
        });
    }

    print_captured_output(capture.output)?;
    Err(io::Error::other("Build script failed"))
}

fn print_captured_output(output: Option<Vec<u8>>) -> io::Result<()> {
    let Some(output) = output else {
        return Ok(());
    };
    eprintln!("\n--- build output ---");
    io::stderr().write_all(&output)?;
    eprintln!("--- end output ---\n");
    Ok(())
}

fn read_stream_in_background<R>(mut stream: R) -> std::thread::JoinHandle<Vec<u8>>
where
    R: io::Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut reader = io::BufReader::new(&mut stream);
        let _ = io::Read::read_to_end(&mut reader, &mut buf);
        buf
    })
}

fn expand_for_launch(value: &str, launch: &BuildLaunch) -> String {
    expand_env_templates(
        value,
        launch.num_cpus,
        &launch.build_dir_str,
        Some(&launch.bootstrap_tools),
        Some(&launch.bootstrap_sysroot),
    )
}

fn unshare_command(launch_script: &str) -> Vec<&str> {
    vec![
        "--user",
        "--pid",
        "--mount",
        "--uts",
        "--fork",
        "--ipc",
        "--net",
        "--map-root-user",
        "/usr/bin/bash",
        "-o",
        "errexit",
        "-o",
        "nounset",
        "-c",
        launch_script,
    ]
}
