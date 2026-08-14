//! Configuration layout for read-only Nex deployments.

use std::fs;
use std::io;
use std::path::Path;

/// Move legacy package defaults out of `/etc` before a Nex deployment is stored.
///
/// The initramfs mounts persistent host state over `/etc` at boot. Programs that
/// do not yet follow UAPI.6 still need pristine defaults, so Nex keeps those
/// files under `/usr/share/factory/etc` and seeds only missing host paths.
pub(super) fn move_legacy_etc_to_factory(target: &Path) -> io::Result<()> {
    let etc = target.join("etc");
    let factory = target.join("usr/share/factory/etc");

    if !etc.exists() {
        fs::create_dir_all(&etc)?;
        return Ok(());
    }

    if factory.exists() {
        merge_without_overwrite(&etc, &factory)?;
        fs::remove_dir(&etc)?;
    } else {
        let parent = factory
            .parent()
            .expect("factory etc path always has a parent");
        fs::create_dir_all(parent)?;
        fs::rename(&etc, &factory)?;
    }

    fs::create_dir(&etc)?;
    println!("  Moved legacy /etc defaults to /usr/share/factory/etc");
    Ok(())
}

fn merge_without_overwrite(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        let source_type = entry.file_type()?;
        let destination_type = match destination_path.symlink_metadata() {
            Ok(metadata) => metadata.file_type(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::rename(&source_path, &destination_path)?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if source_type.is_dir() && destination_type.is_dir() {
            merge_without_overwrite(&source_path, &destination_path)?;
            fs::remove_dir(&source_path)?;
            continue;
        }

        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "legacy /etc default conflicts with native factory file: {}",
                destination_path.display()
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
