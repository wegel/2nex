use super::python_site_packages_dir_prefix;

#[test]
fn finds_python_package_directory_prefix() {
    assert_eq!(
        python_site_packages_dir_prefix("/usr/lib/python3.12/site-packages/requests/__init__.py")
            .as_deref(),
        Some("/usr/lib/python3.12/site-packages/requests/")
    );
    assert_eq!(
        python_site_packages_dir_prefix(
            "/usr/lib/python3.12/site-packages/gi/_gi.cpython-312-x86_64-linux-gnu.so"
        )
        .as_deref(),
        Some("/usr/lib/python3.12/site-packages/gi/")
    );
}

#[test]
fn ignores_top_level_python_module_files() {
    assert_eq!(
        python_site_packages_dir_prefix("/usr/lib/python3.12/site-packages/libvirt.py"),
        None
    );
    assert_eq!(
        python_site_packages_dir_prefix("/usr/lib/libvirt.so.0"),
        None
    );
}
