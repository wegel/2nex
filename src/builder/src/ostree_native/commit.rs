//! OSTree commit object parsing.

use std::collections::HashMap;
use std::io;

use gvariant::{aligned_bytes::copy_to_align, gv, Marker, Structure};

/// parsed commit metadata.
pub struct CommitInfo {
    /// custom metadata key-value pairs (nex.manifest.hash, etc.)
    pub metadata: HashMap<String, String>,
    /// root dirtree checksum (32 bytes as hex)
    pub root_tree: String,
    /// root dirmeta checksum (32 bytes as hex)
    pub root_meta: String,
    /// commit subject
    pub subject: String,
}

/// parse a commit object to extract metadata and root tree checksum.
/// commit format: (a{sv}aya(say)sstayay)
///   - a{sv}: metadata dict
///   - ay: parent commit checksum
///   - a(say): related objects
///   - s: subject
///   - s: body
///   - t: timestamp
///   - ay: root dirtree checksum
///   - ay: root dirmeta checksum
pub fn parse_commit(data: &[u8]) -> io::Result<CommitInfo> {
    // the gvariant crate needs aligned data
    let aligned = copy_to_align(data);

    let commit = gv!("(a{sv}aya(say)sstayay)").cast(aligned.as_ref());
    let (metadata_var, _parent, _related, subject, _body, _timestamp, root_tree, root_meta) =
        commit.to_tuple();

    // parse metadata dict - it's a{sv} where s is key and v is variant
    let mut metadata = HashMap::new();
    for entry in metadata_var {
        let (key, value) = entry.to_tuple();
        let key_str = key.to_str();
        // value is a variant - try to extract string value
        // gvariant::Variant doesn't have a simple to_bytes, need to work around it
        if let Some(val_str) = extract_variant_string_from_variant(&value) {
            metadata.insert(key_str.to_string(), val_str);
        }
    }

    // convert checksums from byte arrays to hex strings
    let root_tree_hex = hex::encode(root_tree.as_ref());
    let root_meta_hex = hex::encode(root_meta.as_ref());

    Ok(CommitInfo {
        metadata,
        root_tree: root_tree_hex,
        root_meta: root_meta_hex,
        subject: subject.to_str().to_string(),
    })
}

/// extract a string from a gvariant Variant.
/// the variant contains type signature followed by value data.
fn extract_variant_string_from_variant(variant: &gvariant::Variant) -> Option<String> {
    // split returns (typestr, data)
    let (typestr, data) = variant.split();

    // check if it's a string type
    if typestr == b"s" {
        // string data is null-terminated in the variant
        let bytes: &[u8] = data.as_ref();
        let str_end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let string_bytes = &bytes[..str_end];

        String::from_utf8(string_bytes.to_vec()).ok()
    } else {
        // not a string type, skip
        None
    }
}

/// get a specific metadata key from commit data.
pub fn get_commit_metadata(data: &[u8], key: &str) -> io::Result<Option<String>> {
    let info = parse_commit(data)?;
    Ok(info.metadata.get(key).cloned())
}
