//! Tests for hook configuration parsing.

use std::path::PathBuf;

use super::{HookConfig, HookPhase, PhaseMode, SourceScope};

#[test]
fn parses_projects_and_phase_modes() {
    let raw = r#"
        [hooks.pre_commit]
        phases = [
          { kind = "code-style", mode = "blocking", scope = "staged" },
          { kind = "clippy", mode = "advisory" },
        ]

        [code_style]
        excluded_paths = ["generated"]

        [[rust_projects]]
        name = "example"
        manifest_path = "src/example/Cargo.toml"
        test_args = ["--no-default-features"]
    "#;

    let config: HookConfig = match toml::from_str(raw) {
        Ok(config) => config,
        Err(error) => panic!("config should parse: {error}"),
    };
    assert_eq!(config.hooks.pre_commit.phases[0].kind, HookPhase::CodeStyle);
    assert_eq!(
        config.hooks.pre_commit.phases[0].scope,
        Some(SourceScope::Staged)
    );
    assert_eq!(config.hooks.pre_commit.phases[1].mode, PhaseMode::Advisory);
    assert_eq!(
        config.code_style.excluded_paths,
        [PathBuf::from("generated")]
    );
    assert_eq!(config.rust_projects[0].test_args, ["--no-default-features"]);
}
