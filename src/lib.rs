use std::{
    borrow::Cow,
    collections::HashSet,
    io,
    path::{Path, PathBuf},
};

/// Extension methods for `std::path::Path` which are correct in the presence of symlinks.
/// These methods are lazy, that is, they preserve as much as possible of the relative and
/// symlinked nature of their arguments, minimally resolving symlinks are necessary to maintain
/// physical path correctness.
pub trait PathExt {
    /// As per `Path::parent` except that it touches the filesystem to ensure that the resulting path
    /// is correct with respect to symlinks.
    ///
    /// Any symlink expansion is minimal, as described above.
    fn real_parent(&self) -> io::Result<Option<Cow<Path>>>;
}

impl PathExt for Path {
    fn real_parent(&self) -> io::Result<Option<Cow<'_, Path>>> {
        if !self.symlink_metadata()?.is_symlink() {
            return Ok(self.parent().map(Cow::Borrowed));
        }

        // we'll have to loop until we find something that's not a symlink,
        // being careful not to get trapped in a cycle of symlinks
        let path = self.to_path_buf();
        let visited: HashSet<PathBuf, _> = HashSet::new();

        loop {
            let target = path.read_link()?;
            if target.is_relative() {
                todo!("relative symlinks not yet implemented");
            }

            let path = target;

            if !path.symlink_metadata()?.is_symlink() {
                return Ok(path.parent().map(|p| Cow::Owned(p.to_path_buf())));
            }
        }
    }
}
