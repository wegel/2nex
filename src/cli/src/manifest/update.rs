use serde_yaml::{Mapping, Value};
use std::fs;
use std::io;
use std::mem;

use super::types::*;

pub fn update_manifest_checksum_field(
    manifest_path: &str,
    kind: ManifestKind,
    new_checksum: &str,
) -> io::Result<()> {
    let contents = fs::read_to_string(manifest_path)?;
    let block_name = match kind {
        ManifestKind::Package => "package",
        ManifestKind::System => "system",
    };
    let header_tag = format!("{block_name}:");
    let mut lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();
    let mut in_section = false;
    let mut replaced = false;
    let mut header_index = None;
    let mut last_section_line = None;
    let mut indent: Option<String> = None;

    for idx in 0..lines.len() {
        let line = lines[idx].clone();
        let trimmed = line.trim();

        if in_section && !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
            in_section = false;
        }

        if !in_section && trimmed == header_tag {
            in_section = true;
            header_index = Some(idx);
            continue;
        }

        if in_section {
            if indent.is_none() && !line.trim().is_empty() {
                indent = Some(
                    line.chars()
                        .take_while(|c| c.is_whitespace())
                        .collect::<String>(),
                );
            }
            last_section_line = Some(idx);
            let trimmed_start = line.trim_start();
            if let Some(rest) = trimmed_start.strip_prefix("checksum:") {
                let trimmed_value = rest.trim();
                let indent_str = indent.clone().unwrap_or_else(|| "  ".to_string());
                let (prefix, suffix) = match trimmed_value.chars().next() {
                    Some('"') if trimmed_value.ends_with('"') && trimmed_value.len() >= 2 => {
                        ("\"".to_string(), "\"".to_string())
                    }
                    Some('\'') if trimmed_value.ends_with('\'') && trimmed_value.len() >= 2 => {
                        ("'".to_string(), "'".to_string())
                    }
                    _ => ("".to_string(), "".to_string()),
                };
                lines[idx] = format!("{indent_str}checksum: {prefix}{new_checksum}{suffix}");
                replaced = true;
                break;
            }
        }
    }

    let header_position = header_index.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Manifest missing {} section", block_name),
        )
    })?;

    if !replaced {
        let insert_after = last_section_line.unwrap_or(header_position);
        let indent_str = indent.unwrap_or_else(|| "  ".to_string());
        lines.insert(
            insert_after + 1,
            format!("{indent_str}checksum: {}", new_checksum),
        );
    }

    let had_trailing_newline = contents.ends_with('\n');
    let mut new_contents = lines.join("\n");
    if had_trailing_newline {
        new_contents.push('\n');
    }
    fs::write(manifest_path, new_contents)?;
    println!(
        "Updated {} checksum in {} to {}",
        block_name, manifest_path, new_checksum
    );
    Ok(())
}

pub fn ensure_output_mapping(value: &mut Value) -> io::Result<(&mut Mapping, bool)> {
    match value {
        Value::Mapping(map) => Ok((map, false)),
        Value::Sequence(_) | Value::Null => {
            let files_seq = if let Value::Sequence(seq) = value {
                mem::take(seq)
            } else {
                Vec::new()
            };
            let mut mapping = Mapping::new();
            mapping.insert(
                Value::String("files".to_string()),
                Value::Sequence(files_seq),
            );
            *value = Value::Mapping(mapping);
            if let Value::Mapping(map) = value {
                Ok((map, true))
            } else {
                unreachable!()
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Output entry must be a sequence or mapping",
        )),
    }
}

pub fn normalize_output_keys(mapping: &mut Mapping) {
    let mut entries: Vec<(Value, Value)> = Vec::new();
    // preserve key ordering: files first, then everything else
    for key in ["files"] {
        let key_value = Value::String(key.to_string());
        if let Some(value) = mapping.remove(&key_value) {
            entries.push((Value::String(key.to_string()), value));
        }
    }
    for (k, v) in mapping.iter() {
        entries.push((k.clone(), v.clone()));
    }
    mapping.clear();
    for (k, v) in entries {
        mapping.insert(k, v);
    }
}

pub fn serialize_outputs_section(outputs_value: &Value) -> io::Result<String> {
    let mapping = outputs_value
        .as_mapping()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "outputs is not a mapping"))?;
    let mut inner = serde_yaml::to_string(&Value::Mapping(mapping.clone()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    if inner.starts_with("---\n") {
        inner = inner[4..].to_string();
    } else if inner.starts_with("---") {
        inner = inner.trim_start_matches("---").trim_start().to_string();
    }
    if inner.ends_with("\n...\n") {
        inner.truncate(inner.len() - 5);
    } else if inner.ends_with("\n...") {
        inner.truncate(inner.len() - 4);
    }
    inner = inner.trim_end().to_string();

    let mut block = String::from("outputs:\n");
    for line in inner.lines() {
        if line.is_empty() {
            block.push('\n');
        } else {
            block.push_str("  ");
            block.push_str(line);
            block.push('\n');
        }
    }

    Ok(block)
}

pub fn locate_outputs_block(contents: &str) -> io::Result<(usize, usize)> {
    let mut start = None;
    let mut end = contents.len();
    let mut line_start = 0;

    while line_start < contents.len() {
        let line_end = contents[line_start..]
            .find('\n')
            .map(|idx| line_start + idx + 1)
            .unwrap_or(contents.len());
        let line = &contents[line_start..line_end];
        let trimmed = line.trim_end();

        if start.is_none() {
            if !line.starts_with(' ') && !line.starts_with('\t') && trimmed == "outputs:" {
                start = Some(line_start);
            }
        } else {
            let trimmed_ws = trimmed.trim();
            let is_top_level =
                !line.starts_with(' ') && !line.starts_with('\t') && !trimmed_ws.is_empty();
            if is_top_level && trimmed_ws != "outputs:" {
                end = line_start;
                break;
            }
        }

        if line_end == contents.len() {
            break;
        }
        line_start = line_end;
    }

    if let Some(start_idx) = start {
        Ok((start_idx, end))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unable to locate outputs section",
        ))
    }
}

/// Write auto-detected outputs to manifest, replacing the outputs section.
pub fn write_auto_outputs_to_manifest(
    manifest_path: &str,
    categorized: &std::collections::HashMap<String, Vec<String>>,
) -> io::Result<()> {
    let contents = fs::read_to_string(manifest_path)?;

    // build the outputs Value from categorized files
    let mut outputs_mapping = Mapping::new();

    // sort categories for deterministic output
    let mut categories: Vec<&String> = categorized.keys().collect();
    categories.sort();

    for category in categories {
        let files = categorized.get(category).unwrap();
        let mut output_mapping = Mapping::new();

        // files list - each file is an object with path field (new format)
        let files_seq: Vec<Value> = files
            .iter()
            .map(|f| {
                let mut file_map = Mapping::new();
                file_map.insert(Value::String("path".to_string()), Value::String(f.clone()));
                Value::Mapping(file_map)
            })
            .collect();
        output_mapping.insert(
            Value::String("files".to_string()),
            Value::Sequence(files_seq),
        );

        outputs_mapping.insert(
            Value::String(category.clone()),
            Value::Mapping(output_mapping),
        );
    }

    let outputs_value = Value::Mapping(outputs_mapping);
    let outputs_block = serialize_outputs_section(&outputs_value)?;

    // locate and replace outputs section
    let (start, end) = locate_outputs_block(&contents)?;
    let mut new_contents = String::new();
    new_contents.push_str(&contents[..start]);
    new_contents.push_str(&outputs_block);
    if !outputs_block.ends_with('\n')
        && (end >= contents.len() || contents[start..end].contains('\n'))
    {
        new_contents.push('\n');
    }
    new_contents.push_str(&contents[end..]);

    fs::write(manifest_path, new_contents)?;
    println!(
        "Auto-detected {} output categories written to {}",
        categorized.len(),
        manifest_path
    );

    Ok(())
}

/// update or insert the build.profile field in a manifest
pub fn update_build_profile(manifest_path: &str, new_profile: &[String]) -> io::Result<()> {
    let contents = fs::read_to_string(manifest_path)?;
    let mut lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();

    let mut in_build = false;
    let mut build_indent: Option<String> = None;
    let mut profile_line_idx = None;
    let mut script_line_idx = None;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // detect leaving build section (new top-level key)
        if in_build
            && !line.starts_with(' ')
            && !line.starts_with('\t')
            && !trimmed.is_empty()
            && trimmed != "build:"
        {
            break;
        }

        if trimmed == "build:" {
            in_build = true;
            continue;
        }

        if in_build {
            // capture indent from first non-empty line
            if build_indent.is_none() && !trimmed.is_empty() {
                build_indent = Some(
                    line.chars()
                        .take_while(|c| c.is_whitespace())
                        .collect::<String>(),
                );
            }

            if trimmed.starts_with("profile:") {
                profile_line_idx = Some(idx);
            }
            if trimmed.starts_with("script:") {
                script_line_idx = Some(idx);
            }
        }
    }

    let indent = build_indent.unwrap_or_else(|| "  ".to_string());

    // format as YAML flow array: [item1, item2, ...]
    let profile_yaml = format!("[{}]", new_profile.join(", "));

    if let Some(idx) = profile_line_idx {
        // update existing profile line
        lines[idx] = format!("{}profile: {}", indent, profile_yaml);
    } else if let Some(script_idx) = script_line_idx {
        // insert before script line (profile comes before script in the struct)
        lines.insert(script_idx, format!("{}profile: {}", indent, profile_yaml));
    } else {
        // no script line found - unusual, but just append to build section
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Could not find build section or script field",
        ));
    }

    let had_trailing_newline = contents.ends_with('\n');
    let mut new_contents = lines.join("\n");
    if had_trailing_newline {
        new_contents.push('\n');
    }
    fs::write(manifest_path, new_contents)?;
    println!("Updated build profile in {}", manifest_path);
    Ok(())
}
