use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Display,
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
    fn real_parent(&self) -> Result<Option<Cow<Path>>, Error>;
}

impl PathExt for Path {
    fn real_parent(&self) -> Result<Option<Cow<'_, Path>>, Error> {
        if !self
            .symlink_metadata()
            .with_path_context(self)?
            .is_symlink()
        {
            println!("{:?} not a symlink, returning simple parent", self);
            return Ok(self.parent().map(Cow::Borrowed));
        }

        println!("{:?} is a symlink, looping", self);

        // we'll have to loop until we find something that's not a symlink,
        // being careful not to get trapped in a cycle of symlinks
        let path = self.to_path_buf();
        let visited: HashSet<PathBuf, _> = HashSet::new();

        loop {
            let target = path.read_link().with_path_context(&path)?;
            if target.is_relative() {
                println!("{:?} is a relative symlink, death", self);
                todo!("relative symlinks not yet implemented");
            }

            let path = target;

            if !path
                .symlink_metadata()
                .with_path_context(&path)?
                .is_symlink()
            {
                println!("resolved {:?} not a symlink, returning simple parent", self);
                return Ok(path.parent().map(|p| Cow::Owned(p.to_path_buf())));
            }
            println!("resolved {:?} is a symlink, looping again", self);
        }
    }
}

/// Our error type is an io:Error which includes the path which failed
#[derive(Debug)]
pub struct Error {
    io_error: io::Error,
    path: PathBuf,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on {}", self.io_error, self.path.to_string_lossy())
    }
}

impl std::error::Error for Error {}

trait PathContext<T> {
    fn with_path_context<P>(self, path: P) -> Result<T, Error>
    where
        P: AsRef<Path>;
}

impl<T> PathContext<T> for Result<T, io::Error> {
    fn with_path_context<P>(self, path: P) -> Result<T, Error>
    where
        P: AsRef<Path>,
    {
        self.map_err(|io_error| Error {
            io_error,
            path: path.as_ref().to_path_buf(),
        })
    }
}
