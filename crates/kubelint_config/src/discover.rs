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
