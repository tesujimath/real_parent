use std::{
    borrow::Cow,
    fmt::Display,
    io,
    path::{Component, Path, PathBuf},
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
    fn real_parent(&self) -> Result<Cow<Path>, Error>;
}

impl PathExt for Path {
    fn real_parent(&self) -> Result<Cow<'_, Path>, Error> {
        if self.as_os_str().is_empty() {
            let parent = Path::new("..");
            let _metadata = parent.symlink_metadata().with_path_context(parent)?;
            Ok(parent.into())
        } else {
            let metadata = self.symlink_metadata().with_path_context(self)?;

            if metadata.is_symlink() {
                symlink_parent(self)
            } else if metadata.is_dir() {
                dir_parent(self)
            } else {
                file_parent(self)
            }
        }
    }
}

fn symlink_parent(path: &Path) -> Result<Cow<'_, Path>, Error> {
    println!("symlink_parent({})", path.to_string_lossy());

    // we'll have to recurse until we find something that's not a symlink,
    // TODO be careful not to get trapped in a cycle of symlinks
    let target = path.read_link().with_path_context(path)?;

    // unwrap is safe because the last path component is a symlink
    let symlink_dir = path.parent().unwrap();

    let resolved_target = if target.is_relative() {
        real_join(symlink_dir, &target)?
    } else {
        target
    };

    resolved_target.real_parent().map(|p| p.into_owned().into())
}

fn dir_parent(path: &Path) -> Result<Cow<'_, Path>, Error> {
    let result: Cow<'_, Path> = match path.file_name() {
        Some(_) => path.parent().unwrap().into(),
        None => path.join("..").into(), // TODO check for overflow error
    };

    println!(
        "dir_parent({}) = {}",
        path.to_string_lossy(),
        result.to_string_lossy()
    );

    Ok(result)
}

fn file_parent(path: &Path) -> Result<Cow<'_, Path>, Error> {
    println!("file_parent({})", path.to_string_lossy());

    Ok(path.parent().unwrap().into())
}

// join paths
// TODO maybe this should be public
fn real_join<P1, P2>(origin: P1, other: P2) -> Result<PathBuf, Error>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let origin = origin.as_ref();
    let other = other.as_ref();

    let mut resolving = origin.to_path_buf();

    println!(
        "calculating real_join({}, {})",
        origin.to_string_lossy(),
        other.to_string_lossy()
    );

    for component in other.components() {
        use Component::*;

        match component {
            CurDir => (),
            Prefix(_) | RootDir => {
                panic!("impossible absolute component in relative path {:?}", other)
            }
            ParentDir => {
                println!("calling {}.real_parent()", resolving.to_string_lossy());
                match resolving.as_path().real_parent() {
                    Ok(path) => {
                        resolving = path.to_path_buf();
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            Normal(path_component) => {
                resolving.push(path_component);
            }
        }
    }

    println!(
        "real_join({}, {}) = {}",
        origin.to_string_lossy(),
        other.to_string_lossy(),
        resolving.to_string_lossy()
    );

    Ok(resolving)
}

/// Our error type is an io:Error which includes the path which failed
#[derive(Debug)]
pub enum Error {
    IO(io::Error, PathBuf),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            IO(e, path) => write!(f, "{} on {}", e, path.to_string_lossy()),
        }
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
        self.map_err(|io_error| Error::IO(io_error, path.as_ref().to_path_buf()))
    }
}
