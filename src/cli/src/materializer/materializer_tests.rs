use std::io;

use tempfile::TempDir;

use crate::store::Store;

use super::{
    bundle_materialize_config, materialize, reject_unresolved_dependencies, requested_only_closure,
    MaterializeConfig, MaterializeMode, MaterializeRequest, RuntimeClosure,
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
fn bundle_helper_config_does_not_require_manifest_db() {
    let temp_dir = TempDir::new().unwrap();

    let config = bundle_materialize_config("repo", temp_dir.path(), MaterializeMode::Nex);

    assert!(!config.resolve_deps);
    assert!(config.manifest_db_paths.is_empty());
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

#[test]
fn requested_only_closure_marks_requests_as_roots_for_nex_checkout() {
    let requests = [MaterializeRequest::Bundle {
        commit: "x86_64/pkg/apps/example/1.0/bundles/full".to_string(),
    }];

    let closure = requested_only_closure(&requests);

    assert!(closure.is_root("x86_64/pkg/apps/example/1.0/bundles/full"));
}

fn host_lacks_root_user_namespace_mapping(error: &io::Error) -> bool {
    error.to_string().contains("uid 0 not mapped in namespace")
}

#[test]
fn resolving_materialize_fails_when_root_manifest_is_missing() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path().join("repo");
    if let Err(error) = Store::init(&repo_path) {
        if host_lacks_root_user_namespace_mapping(&error) {
            eprintln!("skipping missing manifest materializer test: host cannot map uid 0");
            return Ok(());
        }
        return Err(error);
    }

    let manifest_db = temp_dir.path().join("empty-pkg-db");
    std::fs::create_dir_all(&manifest_db)?;
    let config = MaterializeConfig {
        repo_path: repo_path.display().to_string(),
        target_dir: temp_dir.path().join("target"),
        manifest_db_paths: vec![manifest_db],
        resolve_deps: true,
        ..Default::default()
    };
    let requests = [MaterializeRequest::Output {
        commit: "x86_64/pkg/apps/example/1.0/outputs/bin".to_string(),
    }];

    let error = materialize(&config, &requests).unwrap_err();
    let message = error.to_string();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(message.contains("manifest for x86_64/pkg/apps/example/1.0/outputs/bin"));
    assert!(message.contains("was included in the runtime closure"));
    Ok(())
}
