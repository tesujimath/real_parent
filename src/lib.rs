use std::{borrow::Cow, io, path::Path};

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
        // TODO make a proper one
        Ok(self.parent().map(Cow::Borrowed))
    }
}
