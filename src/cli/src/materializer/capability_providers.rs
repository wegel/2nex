//! Runtime capability provider flattening for Nex capsules.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use crate::manifest::types::Manifest;
use crate::manifest::ManifestIndex;
use crate::refs::{PackageRef, RefType};

use super::flatten_errors::{missing_bundle_error, missing_output_error};
use super::flatten_export::flatten_library_replacing_path;

pub(super) fn flatten_capability_providers(
    repo_path: &str,
    pkg_dir: &Path,
    fallback_repos: &[PathBuf],
    manifest_index: &ManifestIndex,
    providers: &BTreeMap<String, String>,
    capabilities: &[(String, String)],
    flattened_files: &mut Vec<String>,
) -> io::Result<()> {
    let mut seen = BTreeSet::new();
    for (_needed_file, capability) in capabilities {
        if !seen.insert(capability.clone()) {
            continue;
        }
        flatten_capability_provider(
            repo_path,
            pkg_dir,
            fallback_repos,
            manifest_index,
            providers,
            capability,
            flattened_files,
        )?;
    }
    Ok(())
}

fn flatten_capability_provider(
    repo_path: &str,
    pkg_dir: &Path,
    fallback_repos: &[PathBuf],
    manifest_index: &ManifestIndex,
    providers: &BTreeMap<String, String>,
    capability: &str,
    flattened_files: &mut Vec<String>,
) -> io::Result<()> {
    let Some(provider_ref) = providers.get(capability) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("runtime capability {capability} is not bound by the system assembly"),
        ));
    };
    for file_path in capability_provider_files(provider_ref, capability, manifest_index)? {
        if flatten_library_replacing_path(
            repo_path,
            provider_ref,
            &file_path,
            pkg_dir,
            fallback_repos,
        )? {
            flattened_files.push(file_path);
        }
    }
    Ok(())
}

pub(super) fn capability_provider_files(
    provider_ref: &str,
    capability: &str,
    manifest_index: &ManifestIndex,
) -> io::Result<Vec<String>> {
    let package_ref = PackageRef::parse(provider_ref).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid provider ref {provider_ref}: {error}"),
        )
    })?;
    let manifest = manifest_index
        .get_manifest(&package_ref.namespace, &package_ref.slug)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no manifest was found for provider ref {provider_ref}"),
            )
        })?;
    let output_names = provider_output_names(provider_ref, manifest, &package_ref)?;
    let mut files = BTreeSet::new();
    let mut provides_capability = false;
    for output_name in output_names {
        let Some(output) = manifest.outputs.get(&output_name) else {
            return Err(missing_output_error(&output_name, manifest));
        };
        if output
            .provides
            .iter()
            .any(|provided| provided == capability)
        {
            provides_capability = true;
            for file in capability_files(output, capability, provider_ref)? {
                files.insert(file);
            }
            continue;
        }
        for file in &output.files {
            files.insert(file.path.clone());
        }
    }
    if !provides_capability {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("provider ref {provider_ref} does not provide {capability}"),
        ));
    }
    Ok(files.into_iter().collect())
}

fn capability_files(
    output: &crate::manifest::types::OutputSpec,
    capability: &str,
    provider_ref: &str,
) -> io::Result<Vec<String>> {
    let Some(files) = output.capability_files.get(capability) else {
        return Ok(output.files.iter().map(|file| file.path.clone()).collect());
    };
    let output_files = output
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    for file in files {
        if !output_files.contains(file.as_str()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "provider ref {provider_ref} declares {file} for {capability}, but the file is not in that output"
                ),
            ));
        }
    }
    Ok(files.clone())
}

fn provider_output_names(
    provider_ref: &str,
    manifest: &Manifest,
    package_ref: &PackageRef,
) -> io::Result<Vec<String>> {
    match &package_ref.ref_type {
        RefType::Output { name, .. } => Ok(vec![name.clone()]),
        RefType::Bundle { name, .. } => manifest
            .bundles
            .get(name)
            .map(|bundle| bundle.includes.clone())
            .ok_or_else(|| missing_bundle_error(name, manifest)),
        RefType::Files { .. } => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("provider ref {provider_ref} must be an output or bundle ref"),
        )),
    }
}

#[cfg(test)]
#[path = "capability_providers_tests.rs"]
mod capability_providers_tests;
