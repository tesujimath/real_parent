use lazy_realpath::PathExt;
use std::{
    env::set_current_dir,
    fmt::Debug,
    fs::{self, create_dir},
    path::Path,
};
use tempfile::{tempdir, TempDir};
use test_case::test_case;

#[test_case("A/B/b", Some("A/B"), Some(1))]
#[test_case("A/B/_b", Some("A/B"), Some(2))]
#[test_case("A/B/_a", Some("A"), None)]
// _uniquifier just to make test case function names unique
fn test_real_parent_rel(path: &str, expected: Option<&str>, _uniquifier: Option<i32>) {
    let farm = LinkFarm::new();

    // naming as follows:
    // - directories are capitalised
    // - files are lower-cased
    // - relative symlinks have an underscore prefix
    // - absolute symlinks have an equals prefix

    farm.dir("A");
    farm.dir("A/B");
    farm.file("A/B/b");
    farm.symlink_file("b", "A/B/_b");
    farm.symlink_file("../a", "A/B/_a");
    farm.file("A/a");
    farm.dir("A/C");
    farm.dir("D");
    farm.symlink_dir("../../C", "A/B/_C");

    farm.set_current_dir();

    check_real_parent_ok(path, expected);
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

    fn set_current_dir(&self) {
        set_current_dir(self.tempdir.path()).unwrap();
    }

    fn dir<P>(&self, path: P)
    where
        P: AsRef<Path>,
    {
        create_dir(self.tempdir.path().join(path)).unwrap();
    }

    fn file<P>(&self, path: P)
    where
        P: AsRef<Path>,
    {
        // create_dir(self.tempdir.path().join(path)).unwrap();
        let path = path.as_ref();
        fs::write(
            self.tempdir.path().join(path),
            path.to_string_lossy().as_bytes(),
        )
        .unwrap()
    }

    #[cfg(target_family = "unix")]
    fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) {
        // symlink_dir(original, self.tempdir.path().join(link)).unwrap()
        std::os::unix::fs::symlink(original, self.tempdir.path().join(link)).unwrap()
    }

    #[cfg(target_family = "windows")]
    fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) {
        std::os::windows::fs::symlink_dir(original, self.tempdir.path().join(link)).unwrap()
    }

    #[cfg(target_family = "unix")]
    fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) {
        std::os::unix::fs::symlink(original, self.tempdir.path().join(link)).unwrap()
    }

    #[cfg(target_family = "windows")]
    fn symlink_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) {
        std::os::windows::fs::symlink_file(original, self.tempdir.path().join(link)).unwrap()
    }
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
                assert_eq!(
                    actual.canonicalize().unwrap(),
                    expected.canonicalize().unwrap(),
                    "canonical paths for {:?}",
                    path
                );
            }
            (Some(actual), None) => panic!("expected None got Some({:?})", actual),
            (None, Some(expected)) => panic!("expected Some({:?}) got None", expected),
            (None, None) => (),
        },
        Err(e) => panic!("real_parent({:?}) failed unexpectedly: {}", path, e),
    }
}
