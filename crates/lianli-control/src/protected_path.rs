use anyhow::{ensure, Context, Result};
use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

pub(crate) fn resolve(path: &Path, boundary: &Path, owners: &[u32]) -> Result<PathBuf> {
    ensure!(
        path.is_absolute() && boundary.is_absolute(),
        "Protected paths must be absolute"
    );
    let mut pending = components(path.strip_prefix(boundary)?);
    let mut resolved = boundary.to_path_buf();
    verify(&resolved, owners, true)?;
    let mut links = 0;
    while let Some(part) = pending.pop_front() {
        if part == ".." {
            ensure!(
                resolved != boundary,
                "Protected path left its trusted directory"
            );
            resolved.pop();
            continue;
        }
        resolved.push(part);
        let metadata = fs::symlink_metadata(&resolved)?;
        ensure!(
            owners.contains(&metadata.uid()),
            "Protected path has unexpected ownership: {}",
            resolved.display()
        );
        if metadata.is_symlink() {
            links += 1;
            ensure!(links <= 40, "Protected path has too many symbolic links");
            let target = fs::read_link(&resolved)?;
            resolved.pop();
            if target.is_absolute() {
                let target = target
                    .strip_prefix(boundary)
                    .context("Protected link left its trusted directory")?;
                pending = components(target).into_iter().chain(pending).collect();
                resolved = boundary.to_path_buf();
            } else {
                pending = components(&target).into_iter().chain(pending).collect();
            }
        } else {
            verify(&resolved, owners, !pending.is_empty())?;
        }
    }
    Ok(resolved)
}

fn components(path: &Path) -> VecDeque<std::ffi::OsString> {
    path.components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_owned()),
            Component::ParentDir => Some("..".into()),
            _ => None,
        })
        .collect()
}

fn verify(path: &Path, owners: &[u32], parent: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    // A sticky store directory protects existing entries owned by trusted accounts.
    let protected_store = parent
        && metadata.is_dir()
        && metadata.mode() & 0o1000 != 0
        && metadata.mode() & 0o002 == 0;
    ensure!(
        owners.contains(&metadata.uid()) && (metadata.mode() & 0o022 == 0 || protected_store),
        "Protected paths must have trusted ownership and write permissions: {}",
        path.display()
    );
    ensure!(
        !parent || metadata.is_dir(),
        "Protected path parent is not a directory"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn resolves_store_links_without_accepting_writable_targets_or_link_chains() {
        let root = tempfile::tempdir().unwrap();
        let owners = [unsafe { libc::geteuid() }];
        let store = root.path().join("store");
        fs::create_dir(&store).unwrap();
        fs::set_permissions(&store, fs::Permissions::from_mode(0o1775)).unwrap();
        let target = store.join("unit");
        fs::write(&target, "fixture").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        let link = root.path().join("unit");
        symlink("store/unit", &link).unwrap();
        assert_eq!(resolve(&link, root.path(), &owners).unwrap(), target);
        fs::set_permissions(&store, fs::Permissions::from_mode(0o775)).unwrap();
        assert!(resolve(&link, root.path(), &owners).is_err());
        fs::set_permissions(&store, fs::Permissions::from_mode(0o1775)).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o664)).unwrap();
        assert!(resolve(&link, root.path(), &owners).is_err());
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
        let writable = root.path().join("writable");
        fs::create_dir(&writable).unwrap();
        fs::set_permissions(&writable, fs::Permissions::from_mode(0o777)).unwrap();
        symlink(&target, writable.join("link")).unwrap();
        let chained = root.path().join("chained");
        symlink(writable.join("link"), &chained).unwrap();
        assert!(resolve(&chained, root.path(), &owners).is_err());
        assert!(resolve(&link, root.path(), &[owners[0].wrapping_add(1)]).is_err());
        let outside = root.path().join("outside");
        symlink("../outside", &outside).unwrap();
        assert!(resolve(&outside, root.path(), &owners).is_err());
        let cycle = root.path().join("cycle");
        symlink("cycle", &cycle).unwrap();
        assert!(resolve(&cycle, root.path(), &owners).is_err());
    }
}
