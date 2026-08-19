use std::fs;
use std::os::unix::fs::symlink;

use super::{sorted_capsule_usr_lib_dirs, symlink_flattened_libs_to_usr};

/// Build many package capsules that all ship the same library name. The
/// creation order is the reverse of the sorted order, and the count is large
/// enough that a hashed directory order will not match the sorted order.
fn capsules_with_shared_library(root: &std::path::Path) {
    for index in (0..32).rev() {
        let usr_lib = root
            .join(format!("pkg{index:02}-provider/1.0/cap{index:02}"))
            .join("usr/lib");
        fs::create_dir_all(&usr_lib).expect("capsule usr/lib");
        fs::write(usr_lib.join("libshared.so.1"), format!("{index}")).expect("library file");
    }
}

#[test]
fn the_first_capsule_in_sorted_order_provides_a_shared_library_name() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let nex_pkg = temp_dir.path().join("nex/pkg");
    let target = temp_dir.path().join("target");
    fs::create_dir_all(&target).expect("target dir");
    capsules_with_shared_library(&nex_pkg);

    symlink_flattened_libs_to_usr(&nex_pkg, &target).expect("symlink libraries");

    let link = fs::read_link(target.join("usr/lib/libshared.so.1")).expect("library link");
    assert_eq!(
        link,
        std::path::Path::new("../../nex/pkg/pkg00-provider/1.0/cap00/usr/lib/libshared.so.1"),
        "an unsorted walk would let readdir order pick the provider"
    );
}

#[test]
fn repeated_runs_choose_the_same_provider() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let nex_pkg = temp_dir.path().join("nex/pkg");
    capsules_with_shared_library(&nex_pkg);

    let mut links = Vec::new();
    for run in 0..3 {
        let target = temp_dir.path().join(format!("target{run}"));
        fs::create_dir_all(&target).expect("target dir");
        symlink_flattened_libs_to_usr(&nex_pkg, &target).expect("symlink libraries");
        links.push(fs::read_link(target.join("usr/lib/libshared.so.1")).expect("library link"));
    }

    assert_eq!(links[0], links[1]);
    assert_eq!(links[1], links[2]);
}

#[test]
fn the_dynamic_loader_is_never_flattened() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let nex_pkg = temp_dir.path().join("nex/pkg");
    let target = temp_dir.path().join("target");
    let usr_lib = nex_pkg.join("glibc/2.39/cccc/usr/lib");
    fs::create_dir_all(&usr_lib).expect("capsule usr/lib");
    fs::create_dir_all(&target).expect("target dir");
    fs::write(usr_lib.join("ld-linux-x86-64.so.2"), "loader").expect("loader file");
    symlink(std::path::Path::new("libc.so.6"), usr_lib.join("libc.so")).expect("capsule link");

    symlink_flattened_libs_to_usr(&nex_pkg, &target).expect("symlink libraries");

    assert!(!target.join("usr/lib/ld-linux-x86-64.so.2").exists());
    assert!(target.join("usr/lib/libc.so").symlink_metadata().is_ok());
}

#[test]
fn capsule_directories_come_back_in_path_order() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let nex_pkg = temp_dir.path().join("nex/pkg");
    capsules_with_shared_library(&nex_pkg);

    let capsule_dirs = sorted_capsule_usr_lib_dirs(&nex_pkg);

    assert_eq!(capsule_dirs.len(), 32);
    let mut expected = capsule_dirs.clone();
    expected.sort();
    assert_eq!(
        capsule_dirs, expected,
        "the provider search must not depend on filesystem readdir order"
    );
}
