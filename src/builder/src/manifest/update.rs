use std::fs;
use std::io;
use std::mem;
use serde_yaml::{Mapping, Value};

use super::types::*;
use crate::runtime::scanner::RuntimeScanResult;

pub fn update_manifest_outputs(manifest_path: &str, suggestions: &RuntimeScanResult) -> io::Result<()> {
    let contents = fs::read_to_string(manifest_path)?;
    let mut doc: Value = serde_yaml::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let outputs_value = doc.get_mut("outputs").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Manifest missing outputs section",
        )
    })?;
    let outputs_map = outputs_value
        .as_mapping_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "outputs is not a mapping"))?;

    let mut changed = false;

    for (key, value) in outputs_map.iter_mut() {
        let category = match key.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        let new_requires: Vec<String> = suggestions
            .category_resolved(&category)
            .map(|commits| commits.keys().cloned().collect())
            .unwrap_or_default();

        let (mapping, converted) = ensure_output_mapping(value)?;
        if converted {
            changed = true;
        }
        if update_requires_field(mapping, &new_requires) {
            changed = true;
        }
        normalize_output_keys(mapping);
    }

    if !changed {
        println!(
            "No outputs.requires changes were necessary for {}",
            manifest_path
        );
        return Ok(());
    }

    let outputs_block = serialize_outputs_section(outputs_value)?;
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
        "Updated outputs.requires entries based on runtime scan in {}",
        manifest_path
    );

    Ok(())
}

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

pub fn update_requires_field(mapping: &mut Mapping, new_values: &[String]) -> bool {
    let key = Value::String("requires".to_string());
    let current: Vec<String> = mapping
        .get(&key)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if new_values.is_empty() {
        if mapping.remove(&key).is_some() && !current.is_empty() {
            return true;
        }
        return false;
    }

    if current == new_values {
        return false;
    }

    let seq = Value::Sequence(
        new_values
            .iter()
            .map(|val| Value::String(val.clone()))
            .collect(),
    );
    mapping.insert(key, seq);
    true
}

pub fn normalize_output_keys(mapping: &mut Mapping) {
    let mut entries: Vec<(Value, Value)> = Vec::new();
    for key in ["files", "requires", "suggests"] {
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

pub fn apply_runtime_requires(manifest: &mut Manifest, suggestions: &RuntimeScanResult) {
    for (category, spec) in manifest.outputs.iter_mut() {
        let new_requires: Vec<String> = suggestions
            .category_resolved(category)
            .map(|commits| commits.keys().cloned().collect())
            .unwrap_or_default();
        spec.requires = new_requires;
    }
}

