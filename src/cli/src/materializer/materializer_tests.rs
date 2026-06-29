use super::{MaterializeConfig, MaterializeMode, MaterializeRequest};

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
