use real_parent::PathExt;
#[cfg(target_family = "unix")]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
use std::{
    env::set_current_dir,
    fmt::Debug,
    fs::{self, create_dir, read_link},
    io,
    path::{Component, Path, PathBuf},
    sync::{Mutex, OnceLock},
};
#[cfg(target_family = "windows")]
use std::{
    iter::once,
    os::windows::fs::{symlink_dir, symlink_file},
    path::Prefix,
};
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;

/// Get root directory.
///
/// On Windows, this will be on the same drive as `tempfile::tempdir`.
pub fn root_dir() -> PathBuf {
    use Component::*;

    let tmp = tempdir().unwrap();
    tmp.path()
        .components()
        .filter(|c| matches!(c, Prefix(_) | RootDir))
        .collect::<PathBuf>()
}

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
    contains_absolute_symlinks: bool,
}

impl LinkFarm {
    pub fn new() -> Self {
        Self {
            cwd: cwd(),
            tempdir: tempdir().unwrap(),
            contains_absolute_symlinks: false,
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
    //
    // also record that the farm now contains absolute symlinks
    pub fn symlink_abs<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        link: P,
        original: Q,
    ) -> &mut Self {
        let original = self.tempdir.path().join(original);
        let link = self.tempdir.path().join(link);
        if link.is_dir() {
            symlink_dir(original, link).unwrap()
        } else {
            symlink_file(original, link).unwrap()
        }

        self.contains_absolute_symlinks = true;

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

    /// run the closure with the specified cwd
    fn run_with_cwd<T, R, F, P>(&self, f: F, arg: T, cwd: P) -> R
    where
        F: Fn(T) -> R,
        P: AsRef<Path>,
    {
        self.cwd.set_during(cwd.as_ref(), f, arg)
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
            is_expected_or_alt_path_ok(path, actual, expected, None, true);
        },
        path,
    );

    // test with absolute paths
    let abs_path = farm.absolute(path);
    let abs_expected = farm.absolute(expected);
    let other_dir = tempdir().unwrap();
    farm.run_with_cwd(
        |path| {
            let actual = path.real_parent();
            // if we ascended out of the farm rootdir it's not straightforward to verify the logical path
            // that was returned, so we simply check the canonical version matches what was expected
            let check_logical = actual.as_ref().is_ok_and(|actual| farm.contains(actual));
            is_expected_or_alt_path_ok(
                abs_path.as_path(),
                actual,
                abs_expected.as_path(),
                None,
                check_logical,
            );
        },
        abs_path.as_path(),
        other_dir.path(),
    );

    test_real_parent_with_unc_path(farm, &abs_path, &abs_expected);
}

#[cfg(target_family = "windows")]
fn convert_disk_to_unc<P>(path: P) -> PathBuf
where
    P: AsRef<Path> + Debug,
{
    let mut components = path.as_ref().components();

    let prefix = if let Some(Component::Prefix(prefix)) = components.next() {
        if let Prefix::Disk(d) = prefix.kind() {
            let prefix_path = Path::new(format!(r"\\localhost\{}$", char::from(d)).leak());
            prefix_path.components().next().unwrap()
        } else {
            panic!(
                "can't convert path {:?} to UNC: prefix {:?} is not a disk",
                path,
                prefix.kind()
            )
        }
    } else {
        panic!(
            "can't convert path {:?} to UNC: failed to find prefix",
            path
        )
    };

    once(prefix).chain(components).collect::<PathBuf>()
}

#[cfg(target_family = "windows")]
fn test_real_parent_with_unc_path<P1, P2>(farm: &LinkFarm, abs_path: P1, abs_expected: P2)
where
    P1: AsRef<Path> + Debug,
    P2: AsRef<Path> + Debug,
{
    let unc_path = convert_disk_to_unc(&abs_path);
    let unc_expected = convert_disk_to_unc(&abs_expected);

    farm.run_without(
        |path| {
            let actual = path.real_parent();
            // if we ascended out of the farm rootdir it's not straightforward to verify the logical path
            // that was returned, so we simply check the canonical version matches what was expected
            let check_logical = actual.as_ref().is_ok_and(|actual| farm.contains(actual));

            // if the link farm contains absolute symlinks, we should accept either a disk path (from the absolute symlink) or a UNC path
            is_expected_or_alt_path_ok(
                unc_path.as_path(),
                actual,
                unc_expected.as_path(),
                farm.contains_absolute_symlinks
                    .then_some(abs_expected.as_ref()),
                check_logical,
            );
        },
        unc_path.as_path(),
    );
}

#[cfg(target_family = "unix")]
fn test_real_parent_with_unc_path<P1, P2>(_farm: &LinkFarm, _abs_path: P1, _abs_expected: P2)
where
    P1: AsRef<Path> + Debug,
    P2: AsRef<Path> + Debug,
{
    // nothing to do here, no UNC paths on unix
}

// Check whether we got what was expected, allowing for an alternate expected case.
// It is sufficient for either one to match.
fn is_expected_or_alt_path_ok(
    path: &Path,
    actual: io::Result<PathBuf>,
    expected: &Path,
    alt_expected: Option<&Path>,
    check_logical: bool,
) {
    match actual {
        Ok(actual) => {
            if check_logical
                && (alt_expected.is_none()
                    || alt_expected.is_some_and(|alt_expected| actual != alt_expected))
            {
                assert_eq!(actual, expected, "logical paths for {:?}", path);
            }

            let actual_canonical = actual.canonicalize().unwrap();
            if alt_expected.is_some_and(|alt_expected| {
                actual_canonical != alt_expected.canonicalize().unwrap()
            }) {
                assert_eq!(
                    actual_canonical,
                    expected.canonicalize().unwrap(),
                    "canonical paths for {:?}",
                    path
                );
            }
            println!(
                "verified \"{}\".real_parent() == \"{}\"",
                path.to_string_lossy(),
                actual.to_string_lossy()
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

// check is_real_root() succeeds with expected, with both absolute and relative paths
pub fn check_is_real_root_ok<P>(farm: &LinkFarm, path: P, expected: bool)
where
    P: AsRef<Path> + Debug,
{
    let path: &Path = path.as_ref();

    // so we can see what went wrong in any failing test
    farm.print();

    // test with relative paths
    farm.run_within(
        |path| {
            let actual = path.is_real_root();
            is_expected_ok(path, actual, expected);
        },
        path,
    );

    // test with absolute paths
    let abs_path = farm.absolute(path);
    let other_dir = tempdir().unwrap();
    farm.run_with_cwd(
        |path| {
            let actual = path.is_real_root();
            is_expected_ok(abs_path.as_path(), actual, expected);
        },
        abs_path.as_path(),
        other_dir.path(),
    );

    test_is_real_root_with_unc_path(farm, &abs_path, expected);
}

// check is_real_root() succeeds with expected, with both absolute and relative paths
pub fn check_is_real_root_in_cwd_ok<P1, P2>(cwd: P1, path: P2, expected: bool)
where
    P1: AsRef<Path> + Debug,
    P2: AsRef<Path> + Debug,
{
    // for the mutual exclusion only:
    let farm = LinkFarm::new();

    let path: &Path = path.as_ref();

    // test with relative paths
    farm.run_with_cwd(
        |path| {
            let actual = path.is_real_root();
            is_expected_ok(path, actual, expected);
        },
        path,
        cwd,
    );
}

#[cfg(target_family = "windows")]
fn test_is_real_root_with_unc_path<P>(farm: &LinkFarm, abs_path: P, expected: bool)
where
    P: AsRef<Path> + Debug,
{
    let unc_path = convert_disk_to_unc(&abs_path);

    farm.run_without(
        |path| {
            let actual = path.is_real_root();
            is_expected_ok(unc_path.as_path(), actual, expected);
        },
        unc_path.as_path(),
    );
}

#[cfg(target_family = "unix")]
fn test_is_real_root_with_unc_path<P>(_farm: &LinkFarm, _abs_path: P, _expected: bool)
where
    P: AsRef<Path> + Debug,
{
    // nothing to do here, no UNC paths on unix
} // Check whether we got what was expected

fn is_expected_ok(path: &Path, actual: io::Result<bool>, expected: bool) {
    match actual {
        Ok(actual) => {
            assert_eq!(actual, expected, "{:?}", path);

            println!(
                "verified \"{}\".is_real_root() == \"{}\"",
                path.to_string_lossy(),
                actual
            );
        }
        Err(e) => panic!("is_real_root({:?}) failed unexpectedly: {:?}", path, e),
    }
}
