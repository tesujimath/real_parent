use lazy_realpath::PathExt;
use std::path::{Path, PathBuf};

use test_case::test_case;

#[test_case("a/b/c", "d", "a/b/c/d")]
#[test_case("a/b/c", "../e", "a/b/c/../e")]
fn test_real_join(this: &str, path: &str, expected: &str) {
    assert_eq!(
        AsRef::<Path>::as_ref(this).real_join(path).unwrap(),
        PathBuf::from(expected)
    );
}

// TODO consider using proptest
#[test_case("relatively/anything", "/something/absolute")]
#[test_case("/absolutely/anything", "/another/absolute/thing")]
fn test_real_join_abs(this: &str, abs_path: &str) {
    assert_eq!(
        AsRef::<Path>::as_ref(this).real_join(abs_path).unwrap(),
        PathBuf::from(abs_path)
    );
}

#[test_case("a/b/c", Some("a/b"))]
fn test_real_parent(this: &str, expected: Option<&str>) {
    match (AsRef::<Path>::as_ref(this).real_parent().unwrap(), expected) {
        (Some(actual), Some(expected)) => assert_eq!(actual, Path::new(expected)),
        (Some(actual), None) => panic!("expected None got Some({:?})", actual),
        (None, Some(expected)) => panic!("expected Some({}) got None", expected),
        (None, None) => (),
    }
}
