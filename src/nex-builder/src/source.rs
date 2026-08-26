//! Source staging at a package or assembly build root.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use nex_source::{SourceKind, SourceRequest};

use crate::manifest::{BuildEnvironment, Source, SourceSpec};

pub(crate) fn stage_sources(
    sources: &[Source],
    manifest_path: &Path,
    cache: &Path,
    root: &Path,
    environment: &BuildEnvironment,
) -> io::Result<BTreeMap<String, String>> {
    fs::create_dir_all(cache)?;
    let inputs = root.join(&environment.paths.inputs);
    fs::create_dir_all(&inputs)?;
    validate_stage_names(sources)?;
    let mut variables = BTreeMap::new();
    for (index, source) in sources.iter().enumerate() {
        let acquired = nex_source::acquire(SourceRequest {
            kind: source_kind(source),
            sha256: &source.sha256,
            manifest: manifest_path,
            cache,
        })?;
        let name = stage_name(source)?;
        let staged = inputs.join(name);
        fs::copy(acquired, &staged)?;
        let build_path = build_path(root, environment, name)?;
        variables.insert(format!("SOURCE{index}"), build_path.clone());
        variables.insert(
            format!("SOURCE_{}", source.name.replace('-', "_")),
            build_path,
        );
    }
    Ok(variables)
}

fn validate_stage_names(sources: &[Source]) -> io::Result<()> {
    let mut names = std::collections::BTreeSet::new();
    for source in sources {
        let name = stage_name(source)?;
        if !names.insert(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("more than one source stages as {}", name.to_string_lossy()),
            ));
        }
    }
    Ok(())
}

fn stage_name(source: &Source) -> io::Result<&OsStr> {
    match &source.kind {
        SourceSpec::File { file } => file.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("source file has no name: {}", file.display()),
            )
        }),
        _ => Ok(OsStr::new(&source.name)),
    }
}

fn source_kind(source: &Source) -> SourceKind<'_> {
    match &source.kind {
        SourceSpec::Url { url } => SourceKind::Url(url),
        SourceSpec::File { file } => SourceKind::File(file),
        SourceSpec::CargoLock { cargo_lock } => SourceKind::CargoLock(cargo_lock),
        SourceSpec::GoSum { go_sum } => SourceKind::GoSum(go_sum),
        SourceSpec::ZigZon { zig_zon } => SourceKind::ZigZon(zig_zon),
        SourceSpec::GitBundle { git_bundle } => SourceKind::GitBundle(git_bundle),
    }
}

fn build_path(root: &Path, environment: &BuildEnvironment, name: &OsStr) -> io::Result<String> {
    let path = if environment.execution.chroot {
        Path::new("/").join(&environment.paths.inputs).join(name)
    } else {
        root.join(&environment.paths.inputs).join(name)
    };
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source path is not UTF-8"))
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
