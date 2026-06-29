use std::io;

use tempfile::TempDir;

use super::{
    materialize, reject_unresolved_dependencies, MaterializeConfig, MaterializeMode,
    MaterializeRequest, RuntimeClosure,
};

#[test]
fn test_materialize_config_default() {
    let config = MaterializeConfig::default();
    assert!(!config.repo_path.is_empty());
    assert_eq!(config.mode, MaterializeMode::Flat);
    assert!(config.resolve_deps);
}

#[test]
fn test_materialize_request_commit() {
    let bundle = MaterializeRequest::Bundle {
        commit: "test/commit".to_string(),
    };
    assert_eq!(bundle.commit(), "test/commit");

    let output = MaterializeRequest::Output {
        commit: "test/output".to_string(),
    };
    assert_eq!(output.commit(), "test/output");

    let files = MaterializeRequest::Files {
        commit: "test/files".to_string(),
        paths: vec!["/bin/foo".to_string()],
    };
    assert_eq!(files.commit(), "test/files");
}

#[test]
fn unresolved_dependencies_return_actionable_error() {
    let mut closure = RuntimeClosure::default();
    closure.add_unresolved(
        "/usr/lib/libmissing.so.1",
        "x86_64/pkg/apps/example/1.0/outputs/bin needs /usr/bin/example".to_string(),
    );

    let error = reject_unresolved_dependencies(&closure).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    let message = error.to_string();
    assert!(message.contains("1 unresolved runtime dependency"));
    assert!(message.contains("/usr/lib/libmissing.so.1"));
    assert!(message.contains("needed by: x86_64/pkg/apps/example/1.0/outputs/bin"));
}

#[test]
fn requested_only_materialize_does_not_require_manifest_db() {
    let temp_dir = TempDir::new().unwrap();
    let config = MaterializeConfig {
        repo_path: temp_dir.path().join("missing-repo").display().to_string(),
        target_dir: temp_dir.path().join("target"),
        resolve_deps: false,
        ..Default::default()
    };
    let requests = [MaterializeRequest::Output {
        commit: "x86_64/pkg/apps/example/1.0/outputs/bin".to_string(),
    }];

    let error = materialize(&config, &requests).unwrap_err();

    assert!(!error.to_string().contains("manifest_db_paths is required"));
}
