use std::path::{Path, PathBuf};

const SKIPPED_DIRS: [&str; 4] = ["target", "node_modules", ".git", "mutants.out"];

pub fn carries_state(file: &std::ffi::OsStr) -> bool {
    let name: std::borrow::Cow<'_, str> = file.to_string_lossy();
    [".db", ".db-wal", ".jsonl"]
        .iter()
        .any(|suffix: &&str| name.ends_with(suffix))
}

pub fn counted(workspace: &Path) -> bool {
    let text: std::borrow::Cow<'_, str> = workspace.to_string_lossy();
    !text.contains("/tests/") && !text.contains("temp_test")
}

pub fn project(workspace: &Path) -> String {
    workspace
        .file_name()
        .map_or_else(String::new, |name: &std::ffi::OsStr| {
            name.to_string_lossy().into_owned()
        })
}

pub fn find(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = vec![];
    let mut pending: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path: PathBuf = entry.path();
            let is_dir: bool = entry
                .file_type()
                .is_ok_and(|t: std::fs::FileType| t.is_dir());
            let name: String = entry.file_name().to_string_lossy().into_owned();
            if !is_dir || SKIPPED_DIRS.contains(&name.as_str()) {
                continue;
            }
            if name == ".beads" {
                if path.join("beads.db").exists() && counted(&dir) {
                    found.push(dir.clone());
                }
                continue;
            }
            pending.push(path);
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_and_temp_tests_are_not_counted() {
        let probes: [bool; 4] = [
            counted(Path::new("/home/u/Projects/mine/kindle")),
            counted(Path::new("/home/u/Projects/beads_rust/tests/fixtures/x")),
            counted(Path::new("/home/u/Projects/beads_rust/temp_test_2")),
            counted(Path::new("/home/u/Projects/p/.claude/worktrees/agent-a")),
        ];
        assert_eq!(probes, [true, false, false, true]);
    }

    #[test]
    fn only_the_database_its_log_and_the_jsonl_carry_bead_state() {
        let probes: [bool; 7] = [
            carries_state(std::ffi::OsStr::new("beads.db")),
            carries_state(std::ffi::OsStr::new("beads.db-wal")),
            carries_state(std::ffi::OsStr::new("issues.jsonl")),
            carries_state(std::ffi::OsStr::new("#18187483")),
            carries_state(std::ffi::OsStr::new(".br-db-openers-e5f3.lock")),
            carries_state(std::ffi::OsStr::new("last-touched")),
            carries_state(std::ffi::OsStr::new("beads.db-shm")),
        ];
        assert_eq!(probes, [true, true, true, false, false, false, false]);
    }

    #[test]
    fn the_project_is_the_workspace_directory_name() {
        assert_eq!(
            (
                project(Path::new("/home/u/Projects/mine/kindle")),
                project(Path::new("/"))
            ),
            (String::from("kindle"), String::new())
        );
    }

    #[test]
    fn find_returns_workspaces_with_a_database_and_skips_the_rest() {
        let root: PathBuf =
            std::env::temp_dir().join(format!("kindle-watch-find-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let make = |dir: &str| std::fs::create_dir_all(root.join(dir)).unwrap();
        let db = |dir: &str| std::fs::write(root.join(dir).join("beads.db"), "").unwrap();
        make("a/.beads");
        db("a/.beads");
        make("b/c/.beads");
        db("b/c/.beads");
        make("d/.beads");
        make("e/tests/f/.beads");
        db("e/tests/f/.beads");
        make("g/target/h/.beads");
        db("g/target/h/.beads");
        let found: Vec<PathBuf> = find(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(found, vec![root.join("a"), root.join("b/c")]);
    }
}
