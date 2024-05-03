#![cfg(test)]
use lazy_realpath::{Error, PathExt};
#[cfg(target_family = "unix")]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
#[cfg(target_family = "windows")]
use std::os::windows::fs::{symlink as symlink_dir, symlink as symlink_file};
use std::{
    env::set_current_dir,
    ffi::OsStr,
    fmt::Debug,
    fs::{self, create_dir, read_link},
    io::{stdout, Write},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};
use test_case::test_case;
use walkdir::WalkDir;

// Naming for files and directories in the link farms is as follows:
// - directories are capitalised
// - files are lower-cased
// - relative symlinks have an underscore prefix
// - absolute symlinks have an equals prefix

#[test_case("x", "")]
#[test_case("A/a", "A")]
#[test_case("A/B/b", "A/B")]
#[test_case("A/B/C", "A/B")]
#[test_case("A/B/C/..", "A/B/C/../..")]
#[test_case("A/B/C/.", "A/B"; "trailing dot is ignored")]
#[test_case("A/./B/C", "A/./B"; "intermediate dot remains")]
#[test_case("A/../A/B/C", "A/../A/B"; "intermediate dotdot remains")]
#[test_case("A/.D", "A" ; "hidden directory")]
#[test_case("A/.D/d", "A/.D" ; "file in hidden directory")]
#[test_case("A/.D/.d", "A/.D" ; "hidden file in hidden directory")]
#[test_case("", ".."; "empty path")]
#[test_case(".", ".."; "bare dot")]
#[test_case("..", "../.."; "bare dotdot")]
#[test_case("../../../../../../../../../..", "../../../../../../../../../../.."; "dotdot overflow is ignored")]
fn test_real_parent_files_directories(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.file("x")
        .dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .dir("A/.D")
        .file("A/a")
        .file("A/B/b")
        .file("A/.D/d")
        .file("A/.D/.d");

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/_b", "A/B")]
#[test_case("A/B/_a", "A")]
#[test_case("A/B/C/_a", "A")]
#[test_case("_B/b", "_B")]
#[test_case("A/_A", "..")]
#[test_case("A/B/_A", "")]
#[test_case("A/B/_B", "A")]
#[test_case("_B/.", "A")]
#[test_case("_B/..", "_B/../..")] // we don't attempt to fold away dotdot in base path
#[test_case("_x", "")]
// TODO more test cases
fn test_real_parent_rel_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.file("x")
        .dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .file("A/a")
        .file("A/B/b")
        .symlink_rel("_x", "x")
        .symlink_rel("_B", "A/B")
        .symlink_rel("A/_A", "..")
        .symlink_rel("A/B/_A", "..")
        .symlink_rel("A/B/_B", ".")
        .symlink_rel("A/B/_b", "b")
        .symlink_rel("A/B/_a", "../a")
        .symlink_rel("A/B/C/_a", "../../a");

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/C/_b", "A/B")]
// TODO more test cases
fn test_real_parent_rel_indirect_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .file("A/B/b")
        .symlink_rel("_B", "A/B")
        .symlink_rel("A/B/C/_b", "../../../_B/b");

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/_b", "A/B")]
#[test_case("A/B/_a", "A")]
#[test_case("A/B/_C", "A")]
fn test_real_parent_abs_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b")
        .file("A/a")
        .symlink_abs("A/B/_b", "A/B/b")
        .symlink_abs("A/B/_a", "A/a")
        .symlink_abs("A/B/_C", "A/C");

    check_real_parent_ok(&farm, path, farm.absolute(expected));
}

struct LinkFarm {
    tempdir: TempDir,
}

impl LinkFarm {
    fn new() -> Self {
        Self {
            tempdir: tempdir().unwrap(),
        }
    }

    // change current directory to root of link farm
    fn set_current_dir(&self) -> &Self {
        set_current_dir(self.tempdir.path()).unwrap();

        self
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

fn is_empty<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    AsRef::<OsStr>::as_ref(path.as_ref()).is_empty()
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
    farm.set_current_dir();
    let actual = path.real_parent();
    is_expected_ok(path, actual, expected, true);

    // test with absolute paths
    let other_dir = tempdir().unwrap();
    set_current_dir(other_dir.path()).unwrap();
    let abs_path = farm.absolute(path);
    let abs_expected = farm.absolute(expected);
    let actual = abs_path.real_parent();

    // if we ascended out of the farm rootdir it's not straigtforward to verify the logical path
    // that was returned, so we simply check the canonical version matches what was expected
    let check_logical = actual.as_ref().is_ok_and(|actual| farm.contains(actual));
    is_expected_ok(
        abs_path.as_path(),
        actual,
        abs_expected.as_path(),
        check_logical,
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
            if !is_empty(&actual) {
                assert_eq!(
                    actual.canonicalize().unwrap(),
                    expected.canonicalize().unwrap(),
                    "canonical paths for {:?}",
                    path
                );
            }
        }
        Err(e) => panic!("real_parent({:?}) failed unexpectedly: {:?}", path, e),
    }
}
