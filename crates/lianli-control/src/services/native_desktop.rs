use std::fs;
use std::io::Read;
use std::path::PathBuf;

const UNIT: &str = "lianli-session.service";
const ROOTS: [&str; 4] = [
    "/etc/systemd/user",
    "/usr/local/lib/systemd/user",
    "/usr/lib/systemd/user",
    "/lib/systemd/user",
];

pub fn native_desktop_unit() -> Option<PathBuf> {
    unit_in(&ROOTS.map(PathBuf::from))
}

pub fn native_desktop_startup_installed() -> bool {
    startup_in(&ROOTS.map(PathBuf::from))
}

fn unit_in(roots: &[PathBuf]) -> Option<PathBuf> {
    for root in roots {
        let unit = root.join(UNIT);
        match fs::symlink_metadata(&unit) {
            Ok(_) => return unit.is_file().then_some(unit),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }
    None
}

fn startup_in(roots: &[PathBuf]) -> bool {
    let inspect = || -> std::io::Result<bool> {
        let Some(unit) = unit_in(roots) else {
            return Ok(false);
        };
        let mut contents = String::new();
        fs::File::open(&unit)?
            .take(4097)
            .read_to_string(&mut contents)?;
        if contents.len() > 4096 || !contents.contains("--login-start") {
            return Ok(false);
        }
        let target = unit.canonicalize()?;
        for root in roots {
            let enabled = root.join("default.target.wants").join(UNIT);
            match fs::symlink_metadata(&enabled) {
                Ok(_) => return Ok(enabled.canonicalize()? == target),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(false)
    };
    inspect().unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn accepts_store_backed_units_and_absolute_startup_links_but_respects_masks() {
        let root = tempfile::tempdir().unwrap();
        let roots = [
            root.path().join("etc/systemd/user"),
            root.path().join("usr/lib/systemd/user"),
        ];
        for directory in &roots {
            fs::create_dir_all(directory.join("default.target.wants")).unwrap();
        }
        let stored = root.path().join("stored.service");
        fs::write(
            &stored,
            "[Service]\nExecStart=/nix/store/fixture/bin/lianli-session --login-start\n",
        )
        .unwrap();
        symlink(&stored, roots[0].join(UNIT)).unwrap();
        let enabled = roots[0].join("default.target.wants").join(UNIT);
        symlink(&stored, &enabled).unwrap();
        assert_eq!(unit_in(&roots), Some(roots[0].join(UNIT)));
        assert!(startup_in(&roots));
        fs::remove_file(roots[0].join(UNIT)).unwrap();
        symlink("/dev/null", roots[0].join(UNIT)).unwrap();
        fs::write(roots[1].join(UNIT), "--login-start").unwrap();
        assert!(unit_in(&roots).is_none());
        assert!(!startup_in(&roots));
        fs::remove_file(roots[0].join(UNIT)).unwrap();
        fs::remove_file(enabled).unwrap();
        symlink(
            "../lianli-session.service",
            roots[1].join("default.target.wants").join(UNIT),
        )
        .unwrap();
        assert!(startup_in(&roots));
        fs::write(roots[1].join(UNIT), "ordinary source helper").unwrap();
        assert!(!startup_in(&roots));
    }
}
