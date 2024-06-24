#![doc = include_str!("../README.md")]

use std::{
    borrow::Cow,
    collections::HashSet,
    fmt::Display,
    io,
    path::{Component, Path, PathBuf},
};

/// Extension methods for `std::path::Path` which are correct in the presence of symlinks.
pub trait PathExt {
    /// As per `Path::parent` except that it touches the filesystem to ensure that the resulting path
    /// is correct with respect to symlinks.
    ///
    /// Any symlink expansion is minimal, that is, as much as possible of the relative and
    /// symlinked nature of the receiver is preserved, minimally resolving symlinks are necessary to maintain
    /// physical path correctness.
    /// For example, no attempt is made to fold away dotdot in the path.
    ///
    /// Differences from `Path::parent`
    /// - `Path::new("..").parent() == ""`, which is incorrect, so `Path::new("..").real_parent() == "../.."`
    /// - `Path::new("foo").parent() == ""`, which is not a valid path, so `Path::new("foo").real_parent() == "."`
    /// - where `Path::parent()` returns `None`, `real_parent()` returns self for absolute root path, and appends `..` otherwise
    fn real_parent(&self) -> io::Result<PathBuf>;
}

impl PathExt for Path {
    fn real_parent(&self) -> io::Result<PathBuf> {
        let mut real_path = RealPath::default();
        real_path
            .parent(self)
            .map(|p| {
                // empty is not a valid path, so we return dot
                if p.as_os_str().is_empty() {
                    AsRef::<Path>::as_ref(DOT).to_path_buf()
                } else {
                    p
                }
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

#[derive(Default, Debug)]
struct RealPath {
    symlinks_visited: HashSet<PathBuf>,
}

impl RealPath {
    fn parent(&mut self, path: &Path) -> Result<PathBuf, Error> {
        if path.as_os_str().is_empty() {
            Ok(DOTDOT.into())
        } else {
            // Trailing dot is troublesome.  The problem is, it looks like a directory to symlink_metadata(),
            // but is invisible to file_name().  We mitigate that inconsistency with a light clean, via components().
            let path = path.components().collect::<PathBuf>();
            let metadata = path.symlink_metadata().with_path_context(&path)?;

            let parent = if metadata.is_symlink() {
                self.symlink_parent(&path)
            } else if metadata.is_dir() {
                self.dir_parent(&path)
            } else {
                self.file_parent(&path)
            };

            parent.map(|p| p.into())
        }
    }

    fn symlink_parent(&mut self, path: &Path) -> Result<Cow<'_, Path>, Error> {
        // check we are not in a cycle of twisty little symlinks, all alike
        let symlink_path = path.to_path_buf();
        if self.symlinks_visited.contains(&symlink_path) {
            return Err(Error::Cycle(symlink_path));
        }
        self.symlinks_visited.insert(symlink_path);

        // we'll have to recurse until we find something that's not a symlink,
        let target = path.read_link().with_path_context(path)?;

        // unwrap is safe because the last path component is a symlink
        let symlink_dir = path.parent().unwrap();

        let resolved_target = if target.is_relative() {
            self.real_join(symlink_dir, &target)?
        } else {
            target
        };

        self.parent(resolved_target.as_path()).map(|p| p.into())
    }

    fn dir_parent<'a>(&mut self, path: &'a Path) -> Result<Cow<'a, Path>, Error> {
        match path.file_name() {
            Some(_) => Ok(path.parent().unwrap().into()),

            None => {
                if path == AsRef::<Path>::as_ref(DOT) {
                    Ok(Into::<PathBuf>::into(DOTDOT).into())
                } else {
                    match path.components().last() {
                        None | Some(Component::ParentDir) => {
                            // don't attempt to fold away dotdot in the base path
                            Ok(path.join(DOTDOT).into())
                        }
                        _ => {
                            // parent of root dir is itself
                            Ok(path.into())
                        }
                    }
                }
            }
        }
    }

    fn file_parent<'a>(&self, path: &'a Path) -> Result<Cow<'a, Path>, Error> {
        Ok(path.parent().unwrap().into())
    }

    // join paths
    // TODO maybe this should have a public interface
    fn real_join<P1, P2>(&mut self, origin: P1, other: P2) -> Result<PathBuf, Error>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let origin = origin.as_ref();
        let other = other.as_ref();

        let mut resolving = origin.to_path_buf();

        for component in other.components() {
            use Component::*;

            match component {
                CurDir => (),
                Prefix(_) | RootDir => {
                    panic!(
                        "impossible absolute component in relative path \"{}\"",
                        other.to_string_lossy()
                    )
                }
                ParentDir => match self.parent(resolving.as_path()) {
                    Ok(path) => {
                        resolving = path.to_path_buf();
                    }
                    Err(e) => {
                        return Err(e);
                    }
                },
                Normal(path_component) => {
                    resolving.push(path_component);
                }
            }
        }

        Ok(resolving)
    }
}

/// Our internal error type is an io:Error which includes the path which failed, or a cycle error.
/// Once ErrorKinds are stabilised, we'll be able to return an io:Error with greater fidelity.
/// See https://github.com/rust-lang/rust/issues/86442
#[derive(Debug)]
enum Error {
    IO(io::Error, PathBuf),
    Cycle(PathBuf),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            IO(e, path) => write!(f, "{} on {}", e, path.to_string_lossy()),
            Cycle(path) => write!(f, "symlink cycle detected at {}", path.to_string_lossy()),
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

const DOT: &str = ".";
const DOTDOT: &str = "..";
