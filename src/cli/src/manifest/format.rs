//! manifest formatter: canonicalizes manifest YAML with consistent ordering and style

use serde_yaml::{Mapping, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io;

/// format a manifest file in place
pub fn format_manifest(path: &str) -> io::Result<()> {
    let contents = fs::read_to_string(path)?;
    let formatted = format_manifest_string(&contents)?;
    fs::write(path, formatted)?;
    Ok(())
}

/// format a manifest string, returning the formatted version
pub fn format_manifest_string(contents: &str) -> io::Result<String> {
    // extract version from original text before YAML parsing mangles it
    // (YAML parses "3.10" as float 3.1, losing the trailing zero)
    let original_version = extract_version_from_text(contents);

    let value: Value = serde_yaml::from_str(contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mapping = value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "manifest must be a mapping"))?;

    let is_system = mapping.contains_key(Value::String("system".to_string()));
    let formatted = format_root(mapping, is_system, original_version.as_deref())?;
    Ok(restore_section_item_comments(contents, &formatted))
}

fn restore_section_item_comments(original: &str, formatted: &str) -> String {
    let comments = collect_section_item_comments(original);
    if comments.is_empty() {
        return formatted.to_string();
    }

    let mut output = String::new();
    let mut section = "";

    for line in formatted.lines() {
        if is_top_level_section(line) {
            section = line.trim_end_matches(':');
        }

        if matches!(section, "sources" | "dependencies" | "packages") {
            if let Some(name) = parse_item_name(line) {
                if let Some(lines) = comments.get(&(section.to_string(), name)) {
                    for comment in lines {
                        output.push_str(comment);
                        output.push('\n');
                    }
                }
            }
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}

fn collect_section_item_comments(contents: &str) -> HashMap<(String, String), Vec<String>> {
    let mut comments = HashMap::new();
    let mut section = "";
    let mut pending: Vec<String> = Vec::new();

    for line in contents.lines() {
        if is_top_level_section(line) {
            section = line.trim_end_matches(':');
            pending.clear();
            continue;
        }

        if !matches!(section, "sources" | "dependencies" | "packages") {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            pending.push(trimmed.to_string());
            continue;
        }

        if let Some(name) = parse_item_name(line) {
            if !pending.is_empty() {
                comments.insert((section.to_string(), name), std::mem::take(&mut pending));
            }
            continue;
        }

        if !trimmed.is_empty() {
            pending.clear();
        }
    }

    comments
}

fn is_top_level_section(line: &str) -> bool {
    let trimmed = line.trim_end();
    !trimmed.is_empty()
        && !line.starts_with(' ')
        && !line.starts_with('\t')
        && trimmed.ends_with(':')
        && !trimmed.starts_with('-')
}

fn parse_item_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let name = trimmed.strip_prefix("- name:")?.trim();
    Some(unquote_scalar(name))
}

fn unquote_scalar(value: &str) -> String {
    value.trim_matches('"').trim_matches('\'').to_string()
}

/// extract version string from raw YAML text before parsing
fn extract_version_from_text(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version:") {
            let value = trimmed.strip_prefix("version:")?.trim();
            // remove quotes if present
            let unquoted = value
                .trim_start_matches('"')
                .trim_end_matches('"')
                .trim_start_matches('\'')
                .trim_end_matches('\'');
            return Some(unquoted.to_string());
        }
    }
    None
}

/// check if manifest has unstable checksums (stable_checksum: false)
fn has_unstable_checksum(mapping: &Mapping) -> bool {
    let pkg_key = Value::String("package".to_string());
    let sys_key = Value::String("system".to_string());

    let header = mapping.get(&pkg_key).or_else(|| mapping.get(&sys_key));

    if let Some(Value::Mapping(pkg)) = header {
        let stable_key = Value::String("stable_checksum".to_string());
        if let Some(Value::Bool(false)) = pkg.get(&stable_key) {
            return true;
        }
    }
    false
}

/// check if manifest is in a bootstrap namespace
fn is_bootstrap_namespace(mapping: &Mapping) -> bool {
    let pkg_key = Value::String("package".to_string());
    let sys_key = Value::String("system".to_string());

    let header = mapping.get(&pkg_key).or_else(|| mapping.get(&sys_key));

    if let Some(Value::Mapping(pkg)) = header {
        let ns_key = Value::String("namespace".to_string());
        if let Some(Value::String(ns)) = pkg.get(&ns_key) {
            return ns.contains("bootstrap/");
        }
    }
    false
}

fn format_root(
    mapping: &Mapping,
    is_system: bool,
    original_version: Option<&str>,
) -> io::Result<String> {
    let mut output = String::new();
    let unstable = has_unstable_checksum(mapping);
    let bootstrap_ns = is_bootstrap_namespace(mapping);
    let skip_needs = unstable || bootstrap_ns;

    // top-level section order (system manifests use "system" instead of "package")
    let sections: &[&str] = if is_system {
        &[
            "system",
            "overlays",
            "sources",
            "dependencies",
            "packages",
            "exclude",
            "build",
        ]
    } else {
        &[
            "package",
            "sources",
            "dependencies",
            "build",
            "bundles",
            "outputs",
            "resolution",
        ]
    };

    let mut first = true;
    for &section in sections {
        // skip resolution for unstable or bootstrap manifests
        if skip_needs && section == "resolution" {
            continue;
        }

        let key = Value::String(section.to_string());
        if let Some(value) = mapping.get(&key) {
            if !first {
                output.push('\n');
            }
            first = false;

            match section {
                "package" => output.push_str(&format_package(value, original_version, unstable)?),
                "system" => output.push_str(&format_system(value, original_version, unstable)?),
                "overlays" => output.push_str(&format_overlays(value)?),
                "sources" => output.push_str(&format_sources(value)?),
                "dependencies" => output.push_str(&format_dependencies(value)?),
                "packages" => output.push_str(&format_packages(value)?),
                "exclude" => output.push_str(&format_generic_section("exclude", value)?),
                "build" => output.push_str(&format_build(value)?),
                "bundles" => output.push_str(&format_bundles(value)?),
                "outputs" => output.push_str(&format_outputs(value, skip_needs)?),
                "resolution" => output.push_str(&format_resolution(value)?),
                _ => {}
            }
        }
    }

    if !output.ends_with('\n') {
        output.push('\n');
    }

    Ok(output)
}

fn format_package(
    value: &Value,
    original_version: Option<&str>,
    unstable: bool,
) -> io::Result<String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "package must be a mapping"))?;

    let mut output = String::from("package:\n");

    // field order for package
    let fields = [
        "schema",
        "name",
        "slug",
        "namespace",
        "version",
        "description",
        "homepage",
        "checksum",
        "stable_checksum",
        "seed",
    ];

    for field in fields {
        // skip checksum for unstable manifests
        if unstable && field == "checksum" {
            continue;
        }

        let key = Value::String(field.to_string());
        if let Some(val) = mapping.get(&key) {
            let formatted = if field == "version" {
                format_version(original_version, val)
            } else {
                format_scalar(val)
            };
            output.push_str(&format!("  {}: {}\n", field, formatted));
        }
    }

    Ok(output)
}

fn format_system(
    value: &Value,
    original_version: Option<&str>,
    unstable: bool,
) -> io::Result<String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "system must be a mapping"))?;

    let mut output = String::from("system:\n");

    let fields = [
        "schema",
        "name",
        "slug",
        "version",
        "architecture",
        "boot_method",
        "description",
        "nex_structure",
        "extends",
        "stable_checksum",
        "checksum",
    ];

    for field in fields {
        // skip checksum for unstable manifests
        if unstable && field == "checksum" {
            continue;
        }

        let key = Value::String(field.to_string());
        if let Some(val) = mapping.get(&key) {
            let formatted = if field == "version" {
                format_version(original_version, val)
            } else {
                format_scalar(val)
            };
            output.push_str(&format!("  {}: {}\n", field, formatted));
        }
    }

    Ok(output)
}

fn format_generic_section(name: &str, value: &Value) -> io::Result<String> {
    let mut output = format!("{name}:\n");
    let rendered =
        serde_yaml::to_string(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    for line in rendered.lines() {
        if line == "---" {
            continue;
        }
        output.push_str("  ");
        output.push_str(line);
        output.push('\n');
    }

    Ok(output)
}

/// format version field, quoting only if it looks like a number
fn format_version(original: Option<&str>, parsed: &Value) -> String {
    // use original text if available (preserves "3.10" that would parse as 3.1)
    let version_str = match original {
        Some(s) => s.to_string(),
        None => match parsed {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => format_scalar(parsed),
        },
    };

    // quote if it looks like a number (could lose precision on re-parse)
    if version_str.parse::<f64>().is_ok() {
        format!("\"{}\"", version_str)
    } else {
        version_str
    }
}

fn format_overlays(value: &Value) -> io::Result<String> {
    let seq = value
        .as_sequence()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "overlays must be a sequence"))?;

    let mut output = String::from("overlays:\n");
    for overlay in seq {
        output.push_str(&format!("- {}\n", format_scalar(overlay)));
    }
    Ok(output)
}

fn format_sources(value: &Value) -> io::Result<String> {
    // handle null or empty sources
    if value.is_null() {
        return Ok(String::from("sources: []\n"));
    }
    let seq = match value.as_sequence() {
        Some(s) if s.is_empty() => return Ok(String::from("sources: []\n")),
        Some(s) => s,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sources must be a sequence",
            ))
        }
    };

    let mut output = String::from("sources:\n");

    for source in seq {
        if let Some(mapping) = source.as_mapping() {
            output.push_str("- ");
            let mut first = true;

            // order: name, url, file, dev, sha256, then auto-vendoring fields
            for field in [
                "name",
                "url",
                "file",
                "dev",
                "sha256",
                "cargo_lock",
                "cargo_toml",
                "go_sum",
                "zig_zon",
            ] {
                let key = Value::String(field.to_string());
                if let Some(val) = mapping.get(&key) {
                    if first {
                        output.push_str(&format!("{}: {}\n", field, format_scalar(val)));
                        first = false;
                    } else {
                        output.push_str(&format!("  {}: {}\n", field, format_scalar(val)));
                    }
                }
            }
        }
    }

    Ok(output)
}

fn format_dependencies(value: &Value) -> io::Result<String> {
    // handle null or empty dependencies
    if value.is_null() {
        return Ok(String::from("dependencies: []\n"));
    }
    let seq = match value.as_sequence() {
        Some(s) if s.is_empty() => return Ok(String::from("dependencies: []\n")),
        Some(s) => s,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "dependencies must be a sequence",
            ))
        }
    };

    let mut output = String::from("dependencies:\n");

    for dep in seq {
        if let Some(mapping) = dep.as_mapping() {
            output.push_str("- ");
            let mut first = true;

            // order: name, commit
            for field in ["name", "commit"] {
                let key = Value::String(field.to_string());
                if let Some(val) = mapping.get(&key) {
                    if first {
                        output.push_str(&format!("{}: {}\n", field, format_scalar(val)));
                        first = false;
                    } else {
                        output.push_str(&format!("  {}: {}\n", field, format_scalar(val)));
                    }
                }
            }
        }
    }

    Ok(output)
}

fn format_packages(value: &Value) -> io::Result<String> {
    let seq = value
        .as_sequence()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "packages must be a sequence"))?;

    let mut output = String::from("packages:\n");

    for pkg in seq {
        if let Some(mapping) = pkg.as_mapping() {
            output.push_str("- ");
            let mut first = true;

            // order: name, commit, outputs
            for field in ["name", "commit", "outputs"] {
                let key = Value::String(field.to_string());
                if let Some(val) = mapping.get(&key) {
                    if first {
                        if field == "outputs" {
                            output.push_str("outputs:\n");
                            if let Some(outputs) = val.as_sequence() {
                                for o in outputs {
                                    output.push_str(&format!("    - {}\n", format_scalar(o)));
                                }
                            }
                        } else {
                            output.push_str(&format!("{}: {}\n", field, format_scalar(val)));
                        }
                        first = false;
                    } else if field == "outputs" {
                        output.push_str("  outputs:\n");
                        if let Some(outputs) = val.as_sequence() {
                            for o in outputs {
                                output.push_str(&format!("    - {}\n", format_scalar(o)));
                            }
                        }
                    } else {
                        output.push_str(&format!("  {}: {}\n", field, format_scalar(val)));
                    }
                }
            }
        }
    }

    Ok(output)
}

fn format_build(value: &Value) -> io::Result<String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "build must be a mapping"))?;

    let mut output = String::from("build:\n");

    // environment comes first
    let env_key = Value::String("environment".to_string());
    if let Some(env) = mapping.get(&env_key) {
        if let Some(s) = env.as_str() {
            output.push_str(&format!("  environment: {}\n", s));
        }
    }

    // profile comes next (if present)
    let profile_key = Value::String("profile".to_string());
    if let Some(profile) = mapping.get(&profile_key) {
        if let Some(seq) = profile.as_sequence() {
            let items: Vec<String> = seq.iter().map(format_scalar).collect();
            output.push_str(&format!("  profile: [{}]\n", items.join(", ")));
        }
    }

    // blank line before script
    let script_key = Value::String("script".to_string());
    if let Some(script) = mapping.get(&script_key) {
        if mapping.contains_key(&profile_key) || mapping.contains_key(&env_key) {
            output.push('\n');
        }
        if let Some(s) = script.as_str() {
            output.push_str("  script: |\n");
            for line in s.lines() {
                if line.is_empty() {
                    output.push('\n');
                } else {
                    output.push_str("    ");
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }
    }

    Ok(output)
}

fn format_bundles(value: &Value) -> io::Result<String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bundles must be a mapping"))?;

    let mut output = String::from("bundles:\n");

    // sort bundle names alphabetically
    let mut bundle_names: Vec<&str> = mapping.keys().filter_map(|k| k.as_str()).collect();
    bundle_names.sort();

    let mut first = true;
    for name in bundle_names {
        let key = Value::String(name.to_string());
        if let Some(val) = mapping.get(&key) {
            if !first {
                output.push('\n');
            }
            first = false;

            output.push_str(&format!("  {}:\n", name));
            if let Some(seq) = val.as_sequence() {
                // sort output references alphabetically
                let mut items: Vec<&str> = seq.iter().filter_map(|v| v.as_str()).collect();
                items.sort();
                for item in items {
                    output.push_str(&format!("  - {}\n", item));
                }
            } else if let Some(map) = val.as_mapping() {
                // handle includes format
                let includes_key = Value::String("includes".to_string());
                if let Some(includes) = map.get(&includes_key) {
                    if let Some(seq) = includes.as_sequence() {
                        output.push_str("    includes:\n");
                        let mut items: Vec<&str> = seq.iter().filter_map(|v| v.as_str()).collect();
                        items.sort();
                        for item in items {
                            output.push_str(&format!("    - {}\n", item));
                        }
                    }
                }
            }
        }
    }

    Ok(output)
}

fn format_outputs(value: &Value, skip_needs: bool) -> io::Result<String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "outputs must be a mapping"))?;

    let mut output = String::from("outputs:\n");

    // sort output names alphabetically
    let mut output_names: Vec<&str> = mapping.keys().filter_map(|k| k.as_str()).collect();
    output_names.sort();

    let mut first = true;
    for name in output_names {
        let key = Value::String(name.to_string());
        if let Some(val) = mapping.get(&key) {
            if !first {
                output.push('\n');
            }
            first = false;

            output.push_str(&format!("  {}:\n", name));

            if let Some(out_map) = val.as_mapping() {
                let files_key = Value::String("files".to_string());
                if let Some(files) = out_map.get(&files_key) {
                    output.push_str("    files:\n");
                    if let Some(seq) = files.as_sequence() {
                        // collect file entries with their paths for sorting
                        let mut file_entries: Vec<(String, &Value)> = seq
                            .iter()
                            .filter_map(|f| {
                                if let Some(map) = f.as_mapping() {
                                    let path_key = Value::String("path".to_string());
                                    if let Some(path) = map.get(&path_key) {
                                        if let Some(p) = path.as_str() {
                                            return Some((p.to_string(), f));
                                        }
                                    }
                                }
                                None
                            })
                            .collect();

                        // sort by path using natural sort
                        file_entries.sort_by(|a, b| natural_cmp(&a.0, &b.0));

                        for (_, file_val) in file_entries {
                            if let Some(file_map) = file_val.as_mapping() {
                                let path_key = Value::String("path".to_string());
                                let needs_key = Value::String("needs".to_string());

                                if let Some(path) = file_map.get(&path_key) {
                                    output.push_str(&format!(
                                        "    - path: {}\n",
                                        format_scalar(path)
                                    ));

                                    // skip needs for unstable/bootstrap manifests
                                    if !skip_needs {
                                        if let Some(needs) = file_map.get(&needs_key) {
                                            if let Some(needs_seq) = needs.as_sequence() {
                                                output.push_str("      needs:\n");
                                                // sort needs alphabetically
                                                let mut needs_list: Vec<&str> = needs_seq
                                                    .iter()
                                                    .filter_map(|v| v.as_str())
                                                    .collect();
                                                needs_list.sort_by(|a, b| natural_cmp(a, b));
                                                for need in needs_list {
                                                    output.push_str(&format!("      - {}\n", need));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(output)
}

fn format_resolution(value: &Value) -> io::Result<String> {
    let mapping = value.as_mapping().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "resolution must be a mapping")
    })?;

    let mut output = String::from("resolution:\n");

    // group by target (value), then sort within groups
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for (key, val) in mapping {
        if let (Some(path), Some(target)) = (key.as_str(), val.as_str()) {
            groups
                .entry(target.to_string())
                .or_default()
                .push(path.to_string());
        }
    }

    // sort group names, then paths within each group
    let mut group_names: Vec<&String> = groups.keys().collect();
    group_names.sort();

    for group in group_names {
        if let Some(paths) = groups.get(group) {
            let mut sorted_paths = paths.clone();
            sorted_paths.sort_by(|a, b| natural_cmp(a, b));
            let formatted_group = format_scalar(&Value::String(group.clone()));
            for path in sorted_paths {
                output.push_str(&format!("  {}: {}\n", path, formatted_group));
            }
        }
    }

    Ok(output)
}

/// format a scalar value, quoting strings only when truly needed
fn format_scalar(value: &Value) -> String {
    match value {
        Value::String(s) => {
            let needs_quotes =
                // colon followed by space looks like nested mapping
                s.contains(": ")
                // comment marker
                || s.contains('#')
                // multiline
                || s.contains('\n')
                // leading/trailing whitespace
                || s.starts_with(' ')
                || s.ends_with(' ')
                // special YAML indicators at start of string
                || s.starts_with('@')
                || s.starts_with('*')
                || s.starts_with('&')
                || s.starts_with('!')
                || s.starts_with('|')
                || s.starts_with('>')
                || s.starts_with('%')
                || s.starts_with('"')
                || s.starts_with('\'')
                || s.starts_with('[')
                || s.starts_with('{')
                || s.starts_with('?')
                // reserved words (case-sensitive in YAML 1.2, but be safe)
                || matches!(s.to_lowercase().as_str(), "true" | "false" | "null" | "yes" | "no" | "on" | "off")
                || s == "~"
                // looks like a number
                || s.parse::<f64>().is_ok();

            if needs_quotes {
                // use double quotes, escape internal quotes
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            } else {
                s.clone()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => serde_yaml::to_string(value)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// natural sort comparison (handles numbers in strings)
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // extract numbers and compare numerically
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();

                    let a_val: u64 = a_num.parse().unwrap_or(0);
                    let b_val: u64 = b_num.parse().unwrap_or(0);

                    match a_val.cmp(&b_val) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    a_chars.next();
                    b_chars.next();
                    match ac.cmp(&bc) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort() {
        let mut items = vec!["file10", "file2", "file1", "file20"];
        items.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(items, vec!["file1", "file2", "file10", "file20"]);
    }

    #[test]
    fn test_natural_sort_paths() {
        let mut items = vec![
            "/usr/lib/libc.so.6",
            "/usr/lib/libc.so.10",
            "/usr/lib/libc.so.2",
        ];
        items.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            items,
            vec![
                "/usr/lib/libc.so.2",
                "/usr/lib/libc.so.6",
                "/usr/lib/libc.so.10",
            ]
        );
    }

    #[test]
    fn formats_system_overlays() {
        let input = r#"system:
  schema: 1
  name: test system
  slug: test
  version: 1.0
overlays:
  - asm/test-overlay.yaml
dependencies: []
packages: []
build:
  environment: env/test.yaml
  script: "true"
"#;

        let formatted = format_manifest_string(input).unwrap();
        assert!(formatted.contains("overlays:\n- asm/test-overlay.yaml\n"));
        assert!(formatted.contains("\ndependencies: []\n"));
    }

    #[test]
    fn formats_system_extends() {
        let input = r#"system:
  schema: 1
  name: child system
  slug: child
  version: 1.0
  description: Child assembly
  nex_structure: true
  extends: asm/base.yaml
  checksum: abc123
packages: []
build:
  environment: env/test.yaml
  script: "true"
"#;

        let formatted = format_manifest_string(input).unwrap();
        let system = formatted.split("\n\n").next().unwrap();
        assert!(system.contains("  nex_structure: true\n"));
        assert!(system.contains("  extends: asm/base.yaml\n"));
        assert!(system.ends_with("  checksum: abc123"));
    }

    #[test]
    fn preserves_system_item_comments() {
        let input = r#"system:
  schema: 1
  name: test system
  slug: test
  version: 1.0
dependencies:
# runtime libs
- name: glibc
  commit: x86_64/pkg/libs/system/glibc/2.39/outputs/lib
packages:
# init tools
- name: systemd
  commit: x86_64/pkg/core/init/systemd/257.5/bundles/minimal
# shell
- name: bash
  commit: x86_64/pkg/cli/shells/bash/5.2.21/outputs/bin
build:
  environment: env/test.yaml
  script: "true"
"#;

        let formatted = format_manifest_string(input).unwrap();
        assert!(formatted.contains("dependencies:\n# runtime libs\n- name: glibc\n"));
        assert!(formatted.contains("packages:\n# init tools\n- name: systemd\n"));
        assert!(formatted.contains("# shell\n- name: bash\n"));
    }
}
