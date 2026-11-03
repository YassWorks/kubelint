//! Finding the configuration file.

use std::path::{Path, PathBuf};

/// The names of a configuration file, in the order they are looked for in each
/// directory.
pub const FILE_NAMES: [&str; 2] = ["kubelint.toml", ".kubelint.toml"];

/// The configuration file that applies to `start`: the first of [`FILE_NAMES`] found in
/// `start` or the nearest directory above it. `None` when there is none, in which case
/// the defaults apply.
///
/// `start` should be absolute (the working directory), so that the search can reach the
/// directories above it.
pub fn find(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|dir| {
        FILE_NAMES
            .iter()
            .map(|name| dir.join(name))
            .find(|path| path.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary directory with the given empty files, removed on drop.
    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str, files: &[&str]) -> Self {
            let root = std::env::temp_dir().join(format!("kubelint-config-{name}"));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("a/b/c")).expect("mkdir");
            for file in files {
                std::fs::write(root.join(file), "").expect("write");
            }
            Self(root)
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn nearest_directory_wins() {
        let tree = Tree::new("nearest", &["kubelint.toml", "a/b/.kubelint.toml"]);
        let found = find(&tree.0.join("a/b/c"));
        assert_eq!(found, Some(tree.0.join("a/b/.kubelint.toml")));
        assert_eq!(find(&tree.0.join("a")), Some(tree.0.join("kubelint.toml")));
    }

    #[test]
    fn visible_name_wins_in_the_same_directory() {
        let tree = Tree::new("both", &["a/kubelint.toml", "a/.kubelint.toml"]);
        assert_eq!(
            find(&tree.0.join("a/b")),
            Some(tree.0.join("a/kubelint.toml"))
        );
    }

    #[test]
    fn a_directory_with_the_name_is_not_a_config_file() {
        let tree = Tree::new("dir", &[]);
        std::fs::create_dir_all(tree.0.join("a/b/kubelint.toml")).expect("mkdir");
        // The search goes on above the tree, where the system temp directory normally has
        // no configuration; comparing with a search from the tree root keeps this robust.
        assert_eq!(find(&tree.0.join("a/b")), find(&tree.0));
    }
}
