//! Build-script execution inside unprivileged Linux namespaces.

use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::manifest::BuildEnvironment;
use crate::progress::{BuildPass, JobProgress};

// --- Per-job output ---

pub(crate) struct JobOutput {
    log: PathBuf,
    progress: Arc<JobProgress>,
}

impl JobOutput {
    pub(crate) fn new(log: PathBuf, progress: Arc<JobProgress>) -> Self {
        Self { log, progress }
    }

    pub(crate) fn begin_pass(&self, pass: BuildPass) -> io::Result<()> {
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log)?;
        writeln!(log, "\n== build {} ==", pass.index() + 1)?;
        self.progress.begin_pass(pass);
        Ok(())
    }

    pub(crate) fn end_pass(&self, pass: BuildPass) {
        self.progress.end_pass(pass);
    }

    pub(crate) fn recorded_profile(&self) -> Option<Vec<String>> {
        self.progress.recorded_profile()
    }
}

// --- Sandbox execution ---

pub(crate) fn run(
    root: &Path,
    script: &str,
    environment: &BuildEnvironment,
    sources: &BTreeMap<String, String>,
    cpu_count: NonZeroUsize,
    output: Option<&JobOutput>,
    pass: BuildPass,
) -> io::Result<()> {
    if let Some(output) = output {
        output.begin_pass(pass)?;
    }
    let result = (|| {
        let root = root.canonicalize()?;
        let script_path = write_script(&root, script)?;
        let variables = environment_variables(&root, environment, sources, cpu_count)?;
        let launch = launch_script(&root, &script_path, environment, cpu_count)?;

        let mut command = sandbox_command(&launch, variables);
        let status = if let Some(output) = output {
            run_captured(&mut command, output)?
        } else {
            command.status()?
        };
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "build script exited with {status}"
            )))
        }
    })();
    if let Some(output) = output {
        output.end_pass(pass);
    }
    result
}

fn sandbox_command(launch: &str, variables: BTreeMap<String, String>) -> Command {
    let mut command = Command::new("unshare");
    command
        .args([
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
            launch,
        ])
        .env_clear()
        .envs(variables);
    command
}

// --- Captured output ---

fn run_captured(command: &mut Command, output: &JobOutput) -> io::Result<std::process::ExitStatus> {
    let (stream, output_stream) = io::pipe()?;
    let error_stream = output_stream.try_clone()?;
    command.stdout(output_stream).stderr(error_stream);
    let log = open_log(&output.log)?;
    let mut child = command.spawn()?;
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    if let Err(error) = pump(stream, log, &output.progress) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    child.wait()
}

fn open_log(path: &Path) -> io::Result<std::fs::File> {
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    OpenOptions::new().create(true).append(true).open(path)
}

fn pump(input: impl io::Read, mut log: std::fs::File, progress: &JobProgress) -> io::Result<()> {
    let mut input = input;
    let mut buffer = [0; 8192];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }
        log.write_all(&buffer[..count])?;
        progress.output(&buffer[..count]);
    }
}

// --- Script environment ---

fn write_script(root: &Path, script: &str) -> io::Result<std::path::PathBuf> {
    let path = root.join("nex/tmp/build.sh");
    fs::create_dir_all(path.parent().expect("script parent"))?;
    fs::write(&path, format!("#!/usr/bin/bash -eu\n{script}\n"))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    Ok(path)
}

fn environment_variables(
    root: &Path,
    environment: &BuildEnvironment,
    sources: &BTreeMap<String, String>,
    cpu_count: NonZeroUsize,
) -> io::Result<BTreeMap<String, String>> {
    let cpu_count = cpu_count.to_string();
    let root = path_text(root)?;
    let mut variables = BTreeMap::new();
    for (name, value) in &environment.env {
        variables.insert(
            name.clone(),
            value
                .replace("{num_cpus}", &cpu_count)
                .replace("{build_dir}", root),
        );
    }
    variables.extend(sources.clone());
    Ok(variables)
}

fn launch_script(
    root: &Path,
    script: &Path,
    environment: &BuildEnvironment,
    cpu_count: NonZeroUsize,
) -> io::Result<String> {
    let quoted_root = shell_quote(path_text(root)?);
    let preamble = environment
        .preamble
        .replace("{build_dir}", &quoted_root)
        .replace("{num_cpus}", &cpu_count.to_string());
    let command = if environment.execution.chroot {
        format!("chroot {quoted_root} /nex/tmp/build.sh")
    } else {
        format!("/usr/bin/bash {}", shell_quote(path_text(script)?))
    };
    Ok(format!("{preamble}\n{command}"))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn path_text(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "build path is not UTF-8"))
}

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod tests;
