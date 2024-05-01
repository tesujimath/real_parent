use lazy_realpath::PathExt;
#[cfg(target_family = "unix")]
use std::os::unix::fs::{symlink as symlink_dir, symlink as symlink_file};
#[cfg(target_family = "windows")]
use std::os::windows::fs::{symlink as symlink_dir, symlink as symlink_file};
use std::{
    env::set_current_dir,
    ffi::OsStr,
    fmt::Debug,
    fs::{self, create_dir},
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};
use test_case::test_case;

// Naming for files and directories in the link farms is as follows:
// - directories are capitalised
// - files are lower-cased
// - relative symlinks have an underscore prefix
// - absolute symlinks have an equals prefix

#[test_case("x", Some(""))]
#[test_case("A/a", Some("A"))]
#[test_case("A/B/b", Some("A/B"))]
#[test_case("A/B/C", Some("A/B"))]
fn test_real_parent_files(path: &str, expected: Option<&str>) {
    let farm = LinkFarm::new();

    farm.file("x")
        .dir("A")
        .dir("A/B")
        .dir("A/B/C")
        .file("A/a")
        .file("A/B/b")
        .set_current_dir();

    check_real_parent_ok(path, expected);
}

#[test_case("A/B/_b", Some("A/B"))]
#[test_case("A/B/_a", Some("A"))]
// TODO more test cases
fn test_real_parent_rel_symlinks(path: &str, expected: Option<&str>) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .file("A/B/b")
        .symlink_rel("b", "A/B/_b")
        .symlink_rel("../a", "A/B/_a")
        .file("A/a")
        .dir("A/C")
        .dir("D")
        .symlink_rel("../../C", "A/B/_C")
        .set_current_dir();

    // TODO remove:
    // std::thread::sleep(std::time::Duration::from_secs(30));

    check_real_parent_ok(path, expected);
}

#[test_case("A/B/_b", Some("A/B"))]
#[test_case("A/B/_a", Some("A"))]
#[test_case("A/B/_C", Some("A"))]
fn test_real_parent_abs_symlinks(path: &str, expected: Option<&str>) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b")
        .file("A/a")
        .symlink_abs("A/B/b", "A/B/_b")
        .symlink_abs("A/a", "A/B/_a")
        .symlink_abs("A/C", "A/B/_C")
        .set_current_dir();

    check_real_parent_ok(path, expected.map(|path| farm.absolute(path)));
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
    fn set_current_dir(&self) {
        set_current_dir(self.tempdir.path()).unwrap();
    }

    // return absolute path within link farm
    fn absolute<P>(&self, path: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.tempdir.path().join(path)
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
    fn symlink_rel<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) -> &Self {
        let link = self.tempdir.path().join(link);
        if link.is_dir() {
            symlink_dir(original, link).unwrap()
        } else {
            symlink_file(original, link).unwrap()
        }

        self
    }

    // create symlink to absolute path in link farm
    fn symlink_abs<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) -> &Self {
        let original = self.tempdir.path().join(original);
        let link = self.tempdir.path().join(link);
        if link.is_dir() {
            symlink_dir(original, link).unwrap()
        } else {
            symlink_file(original, link).unwrap()
        }

        self
    }
}

fn is_empty<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    AsRef::<OsStr>::as_ref(path.as_ref()).is_empty()
}

fn check_real_parent_ok<P1, P2>(path: P1, expected: Option<P2>)
where
    P1: AsRef<Path> + Debug,
    P2: AsRef<Path> + Debug,
{
    let path: &Path = path.as_ref();
    match path.real_parent() {
        Ok(actual) => match (actual, expected) {
            (Some(actual), Some(expected)) => {
                let expected = expected.as_ref();
                assert_eq!(actual, expected, "logical paths for {:?}", path);
                if !is_empty(&actual) {
                    assert_eq!(
                        actual.canonicalize().unwrap(),
                        expected.canonicalize().unwrap(),
                        "canonical paths for {:?}",
                        path
                    );
                }
            }
            (Some(actual), None) => panic!("expected None got Some({:?})", actual),
            (None, Some(expected)) => panic!("expected Some({:?}) got None", expected),
            (None, None) => (),
        },
        Err(e) => panic!("real_parent({:?}) failed unexpectedly: {:?}", path, e),
    }
}
