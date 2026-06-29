//! Source input staging for package and system builds.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::manifest::types::{BuildPaths, Source};
use crate::outputs::fetch_and_verify_input;

/// Fetch declared sources and expose their build-root paths as environment variables.
pub fn handle_inputs(
    sources: &[Source],
    download_dir: &str,
    build_dir: &str,
    use_absolute_paths: bool,
    paths: &BuildPaths,
    canonical_prefix: Option<&str>,
) -> io::Result<HashMap<String, String>> {
    println!("Handling inputs");

    let mut input_env_vars = HashMap::new();

    for (index, source) in sources.iter().enumerate() {
        let input = fetch_and_verify_input(source, download_dir)?;
        let input_path = stage_input_file(&input, build_dir, paths)?;
        let path_str = build_input_path(&input_path, use_absolute_paths, paths, canonical_prefix)?;

        input_env_vars.insert(format!("SOURCE{}", index), path_str.clone());
        insert_named_source_vars(&mut input_env_vars, source, path_str);
    }

    Ok(input_env_vars)
}

fn stage_input_file(
    input: &Path,
    build_dir: &str,
    paths: &BuildPaths,
) -> io::Result<std::path::PathBuf> {
    let inputs_dir = Path::new(build_dir).join(&paths.inputs);
    fs::create_dir_all(&inputs_dir)?;
    let input_path = inputs_dir.join(input.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "source path has no file name")
    })?);
    fs::copy(input, &input_path)?;
    Ok(input_path)
}

fn build_input_path(
    input_path: &Path,
    use_absolute_paths: bool,
    paths: &BuildPaths,
    canonical_prefix: Option<&str>,
) -> io::Result<String> {
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "input path must be UTF-8"))?;

    if use_absolute_paths {
        let prefix = canonical_prefix.unwrap_or("/tmp/bootstrap");
        Ok(format!("{}/{}/{}", prefix, paths.inputs, file_name))
    } else {
        Ok(format!("./{}/{}", paths.inputs, file_name))
    }
}

fn insert_named_source_vars(
    input_env_vars: &mut HashMap<String, String>,
    source: &Source,
    path_str: String,
) {
    let safe_name = source.name.replace('-', "_");
    input_env_vars.insert(format!("SOURCE_{}", safe_name), path_str);

    if source.dev.is_some() {
        input_env_vars.insert(format!("SOURCE_{}_IS_DEV", safe_name), "1".to_string());
    }
}
