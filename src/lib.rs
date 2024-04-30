use std::{
    borrow::Cow,
    io,
    path::{Path, PathBuf},
};

/// Extension methods for `std::path::Path` which are correct in the presence of symlinks.
/// These methods are all lazy, that is, they preserve as much as possible of the relative and
/// symlinked nature of their arguments, minimally resolving symlinks are necessary to maintain
/// physical path correctness.
pub trait PathExt {
    /// As per `Path::join` except that it touches the filesystem to ensure that the resulting path
    /// is correct with respect to symlinks.
    ///
    /// Any symlink expansion is minimal, as described above.
    ///
    /// If `path` is absolute, it is returned as the result regardless of whether it exists in the filesystem.
    fn real_join<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf>;

    /// As per `Path::parent` except that it touches the filesystem to ensure that the resulting path
    /// is correct with respect to symlinks.
    ///
    /// Any symlink expansion is minimal, as described above.
    fn real_parent(&self) -> io::Result<Option<Cow<Path>>>;
}

impl PathExt for Path {
    fn real_join<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        let path = path.as_ref();
        if path.is_absolute() {
            Ok(PathBuf::from(path))
        } else {
            // TODO make a proper one
            Ok(self.join(path))
        }
    }

    fn real_parent(&self) -> io::Result<Option<Cow<'_, Path>>> {
        // TODO make a proper one
        Ok(self.parent().map(Cow::Borrowed))
    }
}
