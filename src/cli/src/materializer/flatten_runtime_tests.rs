use std::io;

use super::flatten_export_error;

#[test]
fn flatten_export_error_names_commit_and_path() {
    let error = flatten_export_error(
        "x86_64/pkg/libs/example/1.0/hash/files",
        "/usr/lib/libmissing.so.1",
        io::Error::new(io::ErrorKind::NotFound, "not in commit"),
    );

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    let message = error.to_string();
    assert!(message.contains("/usr/lib/libmissing.so.1"));
    assert!(message.contains("x86_64/pkg/libs/example/1.0/hash/files"));
    assert!(message.contains("not in commit"));
}
