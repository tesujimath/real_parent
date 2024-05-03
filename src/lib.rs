use std::{
    borrow::Cow,
    collections::HashSet,
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
    /// Any symlink expansion is minimal, as described above.  Therefore no attempt is made to fold away
    /// dotdot in the path.
    ///
    /// Differences from `Path::parent`
    /// - `".."parent() == ""`, which is incorrect, so `"..".real_parent() == "../.."`
    fn real_parent(&self) -> Result<PathBuf, Error>;
}

impl PathExt for Path {
    fn real_parent(&self) -> Result<PathBuf, Error> {
        let mut real_path = RealPath::default();
        real_path.parent(self)
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
        println!("symlink_parent(\"{}\")", path.to_string_lossy());

        // check we are not in a cycle of twisty little symlinks, all alike
        let symlink_path = path.to_path_buf();
        if self.symlinks_visited.contains(&symlink_path) {
            return Err(Error::Cycle(symlink_path));
        } else {
            println!("record visit to symlink {}", symlink_path.to_string_lossy());
            self.symlinks_visited.insert(symlink_path);
        }

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
        let result: Result<Cow<'_, Path>, Error> = match path.file_name() {
            Some(file_name) => {
                println!(
                    "dir_parent(\"{}\") with file_name == \"{}\"",
                    path.to_string_lossy(),
                    file_name.to_string_lossy()
                );
                Ok(path.parent().unwrap().into())
            }
            None => {
                if path == AsRef::<Path>::as_ref(DOT) {
                    println!("dir_parent(\"{}\") is dot", path.to_string_lossy());
                    Ok(Into::<PathBuf>::into(DOTDOT).into())
                } else {
                    println!("dir_parent(\"{}\") ends in dotdot", path.to_string_lossy());
                    // don't attempt to fold away dotdot in the base path
                    Ok(path.join(DOTDOT).into())
                }
            }
        };

        println!("dir_parent(\"{}\") = {:?}", path.to_string_lossy(), result);

        result
    }

    fn file_parent<'a>(&self, path: &'a Path) -> Result<Cow<'a, Path>, Error> {
        println!("file_parent({})", path.to_string_lossy());

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

        println!(
            "calculating real_join(\"{}\", \"{}\")",
            origin.to_string_lossy(),
            other.to_string_lossy()
        );

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
                ParentDir => {
                    println!("calling \"{}\".real_parent()", resolving.to_string_lossy());
                    match self.parent(resolving.as_path()) {
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
            "real_join(\"{}\", \"{}\") = \"{}\"",
            origin.to_string_lossy(),
            other.to_string_lossy(),
            resolving.to_string_lossy()
        );

        Ok(resolving)
    }
}

/// Our error type is an io:Error which includes the path which failed
#[derive(Debug)]
pub enum Error {
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
