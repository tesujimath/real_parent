#![cfg(test)]
use super::*;
use test_case::test_case;

#[test_case("a/b", "c", "a/c")]
#[test_case("a/b/c", "../d", "a/d")]
fn test_resolve_relative_symlink(origin: &str, relpath: &str, expected: &str) {
    assert_eq!(
        resolve_relative_symlink(origin, relpath),
        PathBuf::from(expected)
    );
}
