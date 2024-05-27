use real_parent::PathExt;
#[cfg(target_family = "unix")]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
#[cfg(target_family = "windows")]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::{
    env::set_current_dir,
    fmt::Debug,
    fs::{self, create_dir, read_link},
    io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;

#[derive(Debug)]
struct Cwd {
    mutex: Mutex<()>,
}

impl Cwd {
    fn new() -> Cwd {
        Cwd {
            mutex: Mutex::new(()),
        }
    }

    /// run the closure with cwd set to `path`
    fn set_during<P, T, R, F>(&self, path: P, f: F, arg: T) -> R
    where
        P: AsRef<Path>,
        F: Fn(T) -> R,
    {
        let _guard = match self.mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        set_current_dir(path.as_ref()).unwrap();
        f(arg)
    }
}

fn cwd() -> &'static Cwd {
    static CWD: OnceLock<Cwd> = OnceLock::new();
    CWD.get_or_init(Cwd::new)
}

#[derive(Debug)]
pub struct LinkFarm {
    cwd: &'static Cwd,
    tempdir: TempDir,
}

impl LinkFarm {
    pub fn new() -> Self {
        Self {
            cwd: cwd(),
            tempdir: tempdir().unwrap(),
        }
    }

    // return absolute path within link farm
    pub fn absolute<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.tempdir.path().join(path)
    }

    fn contains<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        path.starts_with(self.tempdir.path())
    }

    // create directory in link farm
    pub fn dir<P>(&self, path: P) -> &Self
    where
        P: AsRef<Path>,
    {
        create_dir(self.tempdir.path().join(path)).unwrap();

        self
    }

    // create file in link farm
    pub fn file<P>(&self, path: P) -> &Self
    where
        P: AsRef<Path>,
    {
        // create_dir(self.tempdir.path().join(path)).unwrap();
        let path = path.as_ref();
        fs::write(
            self.tempdir.path().join(path),
            path.to_string_lossy().as_bytes(),
        )
        .unwrap();

        self
    }

    // create symlink to relative path in link farm
    // note the reversed order of parameters
    pub fn symlink_rel<P: AsRef<Path>, Q: AsRef<Path>>(&self, link: P, original: Q) -> &Self {
        let link = self.tempdir.path().join(link);
        if link.is_dir() {
            symlink_dir(original, link).unwrap()
        } else {
            symlink_file(original, link).unwrap()
        }

        self
    }

    // create symlink to absolute path in link farm
    // note the reversed order of parameters
    pub fn symlink_abs<P: AsRef<Path>, Q: AsRef<Path>>(&self, link: P, original: Q) -> &Self {
        let original = self.tempdir.path().join(original);
        let link = self.tempdir.path().join(link);
        if link.is_dir() {
            symlink_dir(original, link).unwrap()
        } else {
            symlink_file(original, link).unwrap()
        }

        self
    }

    pub fn strip_prefix<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(self.tempdir.path()).unwrap_or(path)
    }

    /// run the closure within the link farm
    fn run_within<T, R, F>(&self, f: F, arg: T) -> R
    where
        F: Fn(T) -> R,
    {
        self.cwd.set_during(self.tempdir.path(), f, arg)
    }

    /// run the closure somewhere other than the link farm
    fn run_without<T, R, F>(&self, f: F, arg: T) -> R
    where
        F: Fn(T) -> R,
    {
        let other_dir = tempdir().unwrap();
        self.cwd.set_during(other_dir.path(), f, arg)
    }

    /// dump the link farm as a diagnostic
    fn print(&self) {
        for entry in WalkDir::new(self.tempdir.path())
            .sort_by_file_name()
            .into_iter()
            .skip(1)
        {
            let entry = entry.unwrap();
            let t = entry.file_type();
            if t.is_dir() {
                println!(
                    "{}/",
                    self.strip_prefix(entry.path()).to_string_lossy().as_ref()
                );
            } else if t.is_file() {
                println!(
                    "{}",
                    self.strip_prefix(entry.path()).to_string_lossy().as_ref()
                )
            } else if t.is_symlink() {
                println!(
                    "{} -> {}",
                    self.strip_prefix(entry.path()).to_string_lossy().as_ref(),
                    read_link(entry.path()).unwrap().to_string_lossy()
                )
            }
        }
        println!();
    }
}

// check real_parent() is as expected, with both absolute and relative paths
pub fn check_real_parent_ok<P1, P2>(farm: &LinkFarm, path: P1, expected: P2)
where
    P1: AsRef<Path> + Debug,
    P2: AsRef<Path> + Debug,
{
    let path: &Path = path.as_ref();
    let expected: &Path = expected.as_ref();

    // so we can see what went wrong in any failing test
    farm.print();

    // test with relative paths
    farm.run_within(
        |path| {
            let actual = path.real_parent();
            is_expected_ok(path, actual, expected, true);
        },
        path,
    );

    // test with absolute paths
    let abs_path = farm.absolute(path);
    let abs_expected = farm.absolute(expected);
    farm.run_without(
        |path| {
            let actual = path.real_parent();
            // if we ascended out of the farm rootdir it's not straigtforward to verify the logical path
            // that was returned, so we simply check the canonical version matches what was expected
            let check_logical = actual.as_ref().is_ok_and(|actual| farm.contains(actual));
            is_expected_ok(
                abs_path.as_path(),
                actual,
                abs_expected.as_path(),
                check_logical,
            );
        },
        abs_path.as_path(),
    );
}

fn is_expected_ok(path: &Path, actual: io::Result<PathBuf>, expected: &Path, check_logical: bool) {
    match actual {
        Ok(actual) => {
            if check_logical {
                assert_eq!(actual, expected, "logical paths for {:?}", path);
            }
            assert_eq!(
                actual.canonicalize().unwrap(),
                expected.canonicalize().unwrap(),
                "canonical paths for {:?}",
                path
            );
        }
        Err(e) => panic!("real_parent({:?}) failed unexpectedly: {:?}", path, e),
    }
}

// check real_parent() returns some kind of error,
// but since real_parent now returns io:Error, we can't distinguish different kinds of failures
pub fn check_real_parent_err<P>(farm: &LinkFarm, path: P)
where
    P: AsRef<Path> + Debug,
{
    let path: &Path = path.as_ref();

    // so we can see what went wrong in any failing test
    farm.print();

    // test with relative paths
    let actual = farm.run_within(|path| path.real_parent(), path);

    if actual.is_ok() {
        panic!(
            "expected error but real_parent({}) succeeded",
            path.to_string_lossy()
        )
    }
}
