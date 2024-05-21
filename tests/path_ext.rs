#![cfg(test)]
use real_parent::{Error, PathExt};
#[cfg(target_family = "unix")]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
#[cfg(target_family = "windows")]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::{
    env::set_current_dir,
    fmt::Debug,
    fs::{self, create_dir, read_link},
    io::{stdout, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tempfile::{tempdir, TempDir};
use test_case::test_case;
use walkdir::WalkDir;

// Naming for files and directories in the link farms is as follows:
// - directories are capitalised
// - files are lower-cased and have a numeric suffix to avoid case insensitivity issues on Windows
// - relative symlinks have an underscore prefix
// - absolute symlinks have an equals prefix

#[test_case("x1", ".")]
#[test_case("A", ".")]
#[test_case("A/a1", "A")]
#[test_case("A/B/b1", "A/B")]
#[test_case("A/B/C", "A/B")]
#[test_case("A/B/C/..", "A/B/C/../..")]
#[test_case("A/B/C/.", "A/B"; "trailing dot is ignored")]
#[test_case("A/./B/C", "A/./B"; "intermediate dot remains")]
#[test_case("A/../A/B/C", "A/../A/B"; "intermediate dotdot remains")]
#[test_case("A/.D", "A" ; "hidden directory")]
#[test_case("A/.D/d1", "A/.D" ; "file in hidden directory")]
#[test_case("A/.D/.d1", "A/.D" ; "hidden file in hidden directory")]
#[test_case("", ".."; "empty path")]
#[test_case(".", ".."; "bare dot")]
#[test_case("..", "../.."; "bare dotdot")]
#[test_case("../../../../../../../../../..", "../../../../../../../../../../.."; "dotdot overflow is ignored")]
fn test_real_parent_files_directories(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.file("x1")
        .dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .dir("A/.D")
        .file("A/a1")
        .file("A/B/b1")
        .file("A/.D/d1")
        .file("A/.D/.d1");

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/_b1", "A/B")]
#[test_case("A/B/_a1", "A")]
#[test_case("A/B/C/_a1", "A")]
#[test_case("_B/b1", "_B")]
#[test_case("A/_dot", "..")]
#[test_case("A/B/_A", ".")]
#[test_case("A/B/_B", "A")]
#[test_case("_B/.", "A")]
#[test_case("_B/..", "_B/../..")] // we don't attempt to fold away dotdot in base path
#[test_case("_x1", ".")]
fn test_real_parent_rel_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.file("x1")
        .dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .file("A/a1")
        .file("A/B/b1")
        .symlink_rel("_x1", "x1")
        .symlink_rel("_B", "A/B")
        .symlink_rel("A/_dot", "..")
        .symlink_rel("A/B/_A", "..")
        .symlink_rel("A/B/_B", ".")
        .symlink_rel("A/B/_b1", "b1")
        .symlink_rel("A/B/_a1", "../a1")
        .symlink_rel("A/B/C/_a1", "../../a1");

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/C/_b", "_B")]
#[test_case("__b", "_B")]
#[test_case("A/B/__c", "A/B/C")]
fn test_real_parent_rel_indirect_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .file("A/B/b1")
        .file("A/B/C/c1")
        .symlink_rel("_B", "A/B")
        .symlink_rel("A/B/C/_b", "../../../_B/b1")
        .symlink_rel("__b", "A/B/C/_b")
        .symlink_rel("_c", "A/B/C/c1")
        .symlink_rel("A/B/__c", "../../_c");

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/_b1", "A/B")]
#[test_case("A/B/_a1", "A")]
#[test_case("A/B/_C", "A")]
fn test_real_parent_abs_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b1")
        .file("A/a1")
        .symlink_abs("A/B/_b1", "A/B/b1")
        .symlink_abs("A/B/_a1", "A/a1")
        .symlink_abs("A/B/_C", "A/C");

    check_real_parent_ok(&farm, path, farm.absolute(expected));
}

#[test_case("A/_a1")]
#[test_case("A/B/_b1")]
fn test_real_parent_symlink_cycle_error(path: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .symlink_rel("A/_a1", "_a2")
        .symlink_rel("A/_a2", "_a1")
        .symlink_rel("A/B/_b1", "../_b2")
        .symlink_rel("A/_b2", "B/_b3")
        .symlink_rel("A/B/_b3", "C/_b4")
        .symlink_rel("A/B/C/_b4", "../_b1");

    check_real_parent_err(&farm, path, ErrorKind::Cycle);
}

#[test_case("X")]
#[test_case("X/y1")]
#[test_case("A/y1")]
#[test_case("_a")]
#[test_case("_b")]
fn test_real_parent_io_error(path: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .file("A/B/b1")
        .symlink_rel("_a", "A/a1")
        .symlink_rel("_b", "A/B/C/b1");

    check_real_parent_err(&farm, path, ErrorKind::Io);
}

#[test_case("_a", "A/A/A")]
fn test_real_parent_symlink_cycle_look_alikes(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/A")
        .dir("A/A/A")
        .file("A/A/A/a1")
        .symlink_rel("_a", "A/_a")
        .symlink_rel("A/_a", "A/_a")
        .symlink_rel("A/A/_a", "A/a1");

    check_real_parent_ok(&farm, path, expected);
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
struct LinkFarm {
    cwd: &'static Cwd,
    tempdir: TempDir,
}

impl LinkFarm {
    fn new() -> Self {
        Self {
            cwd: cwd(),
            tempdir: tempdir().unwrap(),
        }
    }

    // return absolute path within link farm
    fn absolute<P>(&self, path: P) -> PathBuf
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
    fn dir<P>(&self, path: P) -> &Self
    where
        P: AsRef<Path>,
    {
        create_dir(self.tempdir.path().join(path)).unwrap();

        self
    }

    // create file in link farm
    fn file<P>(&self, path: P) -> &Self
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
    fn symlink_rel<P: AsRef<Path>, Q: AsRef<Path>>(&self, link: P, original: Q) -> &Self {
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
    fn symlink_abs<P: AsRef<Path>, Q: AsRef<Path>>(&self, link: P, original: Q) -> &Self {
        let original = self.tempdir.path().join(original);
        let link = self.tempdir.path().join(link);
        if link.is_dir() {
            symlink_dir(original, link).unwrap()
        } else {
            symlink_file(original, link).unwrap()
        }

        self
    }

    fn strip_prefix<'a>(&self, path: &'a Path) -> &'a Path {
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
    fn dump<W>(&self, mut w: W)
    where
        W: Write,
    {
        for entry in WalkDir::new(self.tempdir.path())
            .sort_by_file_name()
            .into_iter()
            .skip(1)
        {
            let entry = entry.unwrap();
            let t = entry.file_type();
            if t.is_dir() {
                writeln!(
                    &mut w,
                    "{}/",
                    self.strip_prefix(entry.path()).to_string_lossy().as_ref()
                )
                .unwrap();
            } else if t.is_file() {
                writeln!(
                    &mut w,
                    "{}",
                    self.strip_prefix(entry.path()).to_string_lossy().as_ref()
                )
                .unwrap();
            } else if t.is_symlink() {
                writeln!(
                    &mut w,
                    "{} -> {}",
                    self.strip_prefix(entry.path()).to_string_lossy().as_ref(),
                    read_link(entry.path()).unwrap().to_string_lossy()
                )
                .unwrap();
            }
        }
        writeln!(&mut w).unwrap()
    }
}

// check real_parent() is as expected, with both absolute and relative paths
fn check_real_parent_ok<P1, P2>(farm: &LinkFarm, path: P1, expected: P2)
where
    P1: AsRef<Path> + Debug,
    P2: AsRef<Path> + Debug,
{
    let path: &Path = path.as_ref();
    let expected: &Path = expected.as_ref();

    // so we can see what went wrong in any failing test
    farm.dump(stdout());

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

fn is_expected_ok(
    path: &Path,
    actual: Result<PathBuf, Error>,
    expected: &Path,
    check_logical: bool,
) {
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

// check real_parent() is the expected error, just for relative path
fn check_real_parent_err<P>(farm: &LinkFarm, path: P, expected_error_kind: ErrorKind)
where
    P: AsRef<Path> + Debug,
{
    let path: &Path = path.as_ref();

    // so we can see what went wrong in any failing test
    farm.dump(stdout());

    // test with relative paths
    let actual = farm.run_within(|path| path.real_parent(), path);

    match actual {
        Ok(_) => panic!(
            "expected {:?} error but real_parent({}) succeeded",
            expected_error_kind,
            path.to_string_lossy()
        ),
        Err(Error::IO(e, error_path)) => assert_eq!(
            ErrorKind::Io,
            expected_error_kind,
            "expected {:?} error but got {} on {}",
            expected_error_kind,
            e,
            error_path.to_string_lossy(),
        ),
        Err(Error::Cycle(error_path)) => assert_eq!(
            ErrorKind::Cycle,
            expected_error_kind,
            "expected {:?} error but got {:?} on {}",
            expected_error_kind,
            ErrorKind::Cycle,
            error_path.to_string_lossy(),
        ),
    }
}

#[derive(PartialEq, Eq, Debug)]
enum ErrorKind {
    Io,
    Cycle,
}
