use lazy_realpath::PathExt;
use std::path::Path;

use test_case::test_case;

#[test_case("a/b/c", Some("a/b"))]
fn test_real_parent(this: &str, expected: Option<&str>) {
    match (AsRef::<Path>::as_ref(this).real_parent().unwrap(), expected) {
        (Some(actual), Some(expected)) => assert_eq!(actual, Path::new(expected)),
        (Some(actual), None) => panic!("expected None got Some({:?})", actual),
        (None, Some(expected)) => panic!("expected Some({}) got None", expected),
        (None, None) => (),
    }
}
