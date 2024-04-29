use std::path::{Path, PathBuf};

/// Extension methods for `std::path::Path` which are correct in the presence of symlinks.
/// These methods are all lazy, that is, they preserve as much as possible of the relative and
/// symlinked nature of their arguments, minimally resolving symlinks are necessary to maintain
/// physical path correctness.
trait PathExt {
    /// As per `Path::join` except that it touches the filesystem to ensure that the resulting path
    /// is correct with respect to symlinks.
    ///
    /// Any symlink expansion is minimal, as described above.
    fn real_join<P: AsRef<Path>>(&self, path: P) -> PathBuf;

    /// As per `Path::parent` except that it touches the filesystem to ensure that the resulting path
    /// is correct with respect to symlinks.
    ///
    /// Any symlink expansion is minimal, as described above.
    fn real_parent(&self) -> Option<&Path>;
}
