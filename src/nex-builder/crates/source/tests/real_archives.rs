//! Optional network checks against source hashes retained by real manifests.

use std::path::{Path, PathBuf};

use nex_source::{acquire, SourceKind, SourceRequest};

#[test]
fn cargo_archive_matches_retained_hash() {
    check(
        SourceKind::CargoLock(
            "https://raw.githubusercontent.com/wegel/zub/17f2ca6d1437a32e4123a9a30ac47a3896254a55/Cargo.lock",
        ),
        "0e2853ae62391172ff1786df7db9cbead20277c8bc788890089db477d2b24033",
        "catalog/pkg/core/nex/zub.yaml",
    );
}

#[test]
fn go_archive_matches_retained_hash() {
    check(
        SourceKind::GoSum("https://raw.githubusercontent.com/FiloSottile/age/v1.2.1/go.sum"),
        "054ce23eb53f326099604caca60473ae87d25ebc4c4248b89b76d34f428fab77",
        "catalog/pkg/cli/crypto/age.yaml",
    );
}

#[test]
fn zig_archive_matches_retained_hash() {
    check(
        SourceKind::ZigZon(
            "https://raw.githubusercontent.com/neurosnap/zmx/4d70b4c97d254aa058e4774b6cb53a3ee9d3992d/build.zig.zon",
        ),
        "940b93da319105e921444d5dd08f942750c718d81b5e08fea7f42c43f32fed8f",
        "catalog/pkg/cli/system/zmx.yaml",
    );
}

fn check(kind: SourceKind<'_>, sha256: &str, manifest: &str) {
    let Some(cache) = test_cache() else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| path.join("catalog").is_dir())
        .expect("repository root");
    let path = acquire(SourceRequest {
        kind,
        sha256,
        manifest: &root.join(manifest),
        cache: &cache,
    })
    .unwrap();
    assert!(path.is_file());
}

fn test_cache() -> Option<PathBuf> {
    std::env::var_os("NEX_SOURCE_NETWORK_TEST").map(PathBuf::from)
}
