use std::path::Path;

use real_parent::PathExt;
use test_case::test_case;

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

    check_path_ok(&farm, None, path, expected, Path::real_parent);
}

#[test]
fn test_real_parent_root_dir() {
    let farm = LinkFarm::new();

    let path = root_dir();
    let expected = path.as_path();
    check_path_ok(&farm, None, &path, expected, Path::real_parent);
}

#[test_case("A/B/_b1", "A/B")]
#[test_case("A/B/_a1", "A")]
#[test_case("A/B/C/_a1", "A")]
#[test_case("A/_dot", "..")]
#[test_case("A/B/_A", ".")]
#[test_case("A/B/_B", "A")]
#[test_case("A/B/C/_b1", "A/B")]
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
        .file("A/B/C/c1")
        .symlink_rel("_x1", "x1")
        .symlink_rel("_B", "A/B")
        .symlink_rel("A/_dot", "..")
        .symlink_rel("A/B/_A", "..")
        .symlink_rel("A/B/_B", ".")
        .symlink_rel("A/B/_b1", "b1")
        .symlink_rel("A/B/_a1", "../a1")
        .symlink_rel("A/B/C/_a1", "../../a1")
        .symlink_rel("A/B/C/_b1", "./.././b1");

    check_path_ok(&farm, None, path, expected, Path::real_parent);
}

#[test_case("_B/b1", "_B")]
#[cfg(not(target_family = "windows"))]
fn test_real_parent_rel_symlinks_not_windows(path: &str, expected: &str) {
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

    check_path_ok(&farm, None, path, expected, Path::real_parent);
}

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

    check_path_ok(&farm, None, path, expected, Path::real_parent);
}

#[test_case("A/B/C/_b", "_B")]
#[test_case("__b", "_B")]
#[cfg(not(target_family = "windows"))]
fn test_real_parent_rel_indirect_symlinks_not_windows(path: &str, expected: &str) {
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

    check_path_ok(&farm, None, path, expected, Path::real_parent);
}

#[test_case("A/B/=b1", "A/B")]
#[test_case("A/B/=a1", "A")]
#[test_case("A/B/=C", "A")]
fn test_real_parent_abs_symlinks(path: &str, expected: &str) {
    let mut farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b1")
        .file("A/a1");

    farm.symlink_abs("A/B/=b1", "A/B/b1")
        .symlink_abs("A/B/=a1", "A/a1")
        .symlink_abs("A/B/=C", "A/C");

    check_path_ok(
        &farm,
        None,
        path,
        farm.absolute(expected),
        Path::real_parent,
    );
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

    // since real_parent now returns io:Error, we can't distinguish different kinds of failures
    check_path_err(&farm, path, Path::real_parent);
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

    // since real_parent now returns io:Error, we can't distinguish different kinds of failures
    check_path_err(&farm, path, Path::real_parent);
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

    check_path_ok(&farm, None, path, expected, Path::real_parent);
}

#[test_case("x1", "x1")]
#[test_case("A", "A")]
#[test_case("A/a1", "A/a1")]
#[test_case("A/B/b1", "A/B/b1")]
#[test_case("A/B/C", "A/B/C")]
#[test_case("A/B/C/..", "A/B")]
#[test_case("./A/B/b1", "A/B/b1"; "initial dot removed")]
#[test_case("A/B/C/.", "A/B/C"; "trailing dot is ignored")]
#[test_case("A/./B/C", "A/B/C"; "intermediate dot removed")]
#[test_case("A/../A/B/C", "A/B/C"; "intermediate dotdot folded away")]
#[test_case("A/.D", "A/.D" ; "hidden directory")]
#[test_case("A/.D/d1", "A/.D/d1" ; "file in hidden directory")]
#[test_case("A/.D/.d1", "A/.D/.d1" ; "hidden file in hidden directory")]
#[test_case("", "."; "empty path")]
#[test_case(".", "."; "bare dot")]
#[test_case("..", ".."; "bare dotdot")]
#[test_case("../../../../../../../../../..", "../../../../../../../../../.."; "dotdot overflow is ignored")]
fn test_real_clean_files_directories(path: &str, expected: &str) {
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

    check_path_ok(&farm, None, path, expected, Path::real_clean);
}

#[test]
fn test_real_clean_root_dir() {
    let farm = LinkFarm::new();

    let path = root_dir();
    let expected = path.as_path();
    check_path_ok(&farm, None, &path, expected, Path::real_clean);
}

#[test_case("C/..", "A/B", ".")]
#[test_case("../..", "A/B/C", "../..")]
#[test_case("../C/../../B", "A/B/C", "../../B")]
#[test_case("../C/../../B/..", "A/B/C", "../..")]
fn test_real_clean_parent(path: &str, cwd: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A").dir("A/B").dir("A/B/C");

    check_path_ok(&farm, Some(cwd), path, expected, Path::real_clean);
}

#[test_case("A/B/_a1/..", "A")]
#[test_case("A/B/_b1/..", "A/B")]
#[test_case("A/B/C/_a1/../B", "A/B")]
#[test_case("A/B/C/_A/../A", "A")]
#[test_case("A/B/C/_B/..", "A")]
#[test_case("A/B/C/_B/../B/C/..", "A/B")]
fn test_real_clean_rel_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
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
        .symlink_rel("A/B/C/_a1", "../../a1")
        .symlink_rel("A/B/C/_A", "../../../A")
        .symlink_rel("A/B/C/_B", "./..");

    check_path_ok(&farm, None, path, expected, Path::real_clean);
}

#[test_case("A/B/C/_A/B", "A/B/C/_A/B")]
// #[cfg(not(target_family = "windows"))]
fn test_real_clean_rel_symlinks_not_windows(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
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
        .symlink_rel("A/B/C/_a1", "../../a1")
        .symlink_rel("A/B/C/_A", "../../../A")
        .symlink_rel("A/B/C/_B", "./..");

    check_path_ok(&farm, None, path, expected, Path::real_clean);
}

#[test_case("A/B/__c/..", "A/B/C")]
fn test_real_clean_rel_indirect_symlinks(path: &str, expected: &str) {
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

    check_path_ok(&farm, None, path, expected, Path::real_clean);
}

#[test_case("A/B/=b1/..", "A/B")]
#[test_case("A/B/=a1/..", "A")]
#[test_case("A/B/=C/..", "A")]
fn test_real_clean_abs_symlinks(path: &str, expected: &str) {
    let mut farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b1")
        .file("A/a1");

    farm.symlink_abs("A/B/=b1", "A/B/b1")
        .symlink_abs("A/B/=a1", "A/a1")
        .symlink_abs("A/B/=C", "A/C");

    check_path_ok(&farm, None, path, farm.absolute(expected), Path::real_clean);
}

#[test]
fn test_is_real_root_root_dir() {
    let root_dir = root_dir();

    let actual = root_dir.as_path().is_real_root().unwrap();
    assert!(actual);
}

#[test_case(""; "empty")]
#[test_case("."; "dot")]
#[test_case(".."; "dotdot")]
fn test_is_real_root_in_root_dir(path: &str) {
    let root_dir = root_dir();

    check_is_real_root_in_cwd_ok(root_dir.as_path(), path, true);
}

#[test]
fn test_is_real_root_ancestor() {
    let farm = LinkFarm::new();
    let farm_depth = farm.depth_below_root();
    let mut candidate = farm.absolute(".");

    // loop until we have ascended to root
    for _i in 0..farm_depth {
        let candidate_path = candidate.as_path();
        assert!(
            !candidate_path.is_real_root().unwrap(),
            "{:?} is not root",
            candidate_path
        );
        candidate = candidate_path.real_parent().unwrap();
    }

    let candidate_path = candidate.as_path();
    assert!(
        candidate_path.is_real_root().unwrap(),
        "{:?} is root",
        candidate_path
    );
}

#[test_case("_r1")]
#[test_case("A/_r2")]
#[test_case("A/_r1a")]
#[test_case("A/_r2a")]
#[cfg(not(target_family = "windows"))]
fn test_is_real_root_via_symlinks_not_windows(path: &str) {
    let root_dir = root_dir();
    let root_path = root_dir.as_path();
    let mut farm = LinkFarm::new();

    farm.dir("A");

    farm.symlink_external("_r1", root_path)
        .symlink_external("A/_r2", root_path)
        .symlink_rel("A/_r1a", "../_r1")
        .symlink_rel("A/_r2a", "_r2");

    check_is_real_root_ok(&farm, path, true);
}

#[test_case("x1")]
#[test_case("A")]
#[test_case("A/a1")]
#[test_case("A/B/b1")]
#[test_case("A/B/C")]
#[test_case("A/B/C/.."; "parent")]
#[test_case("A/B/C/."; "trailing dot is ignored")]
#[test_case("A/./B/C"; "intermediate dot remains")]
#[test_case("A/../A/B/C"; "intermediate dotdot remains")]
#[test_case("A/.D"; "hidden directory")]
#[test_case("A/.D/d1"; "file in hidden directory")]
#[test_case("A/.D/.d1"; "hidden file in hidden directory")]
#[test_case(""; "empty path")]
#[test_case("."; "bare dot")]
#[test_case(".."; "bare dotdot")]
fn test_is_real_root_not_files_directories(path: &str) {
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

    check_is_real_root_ok(&farm, path, false);
}

#[test_case("A/B/_b1")]
#[test_case("A/B/_a1")]
#[test_case("A/B/C/_a1")]
#[test_case("A/_dot")]
#[test_case("A/B/_A")]
#[test_case("A/B/_B")]
#[test_case("_B/."; "dot")]
#[test_case("_B/.."; "dotdot")]
#[test_case("_x1")]
#[test_case("_B/b1")]
#[cfg(not(target_family = "windows"))]
fn test_is_real_root_not_rel_symlinks_not_windows(path: &str) {
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

    check_is_real_root_ok(&farm, path, false);
}

#[test_case("A/B/__c")]
#[test_case("A/B/C/_b")]
#[test_case("__b")]
#[cfg(not(target_family = "windows"))]
fn test_is_real_root_not_rel_indirect_symlinks_not_windows(path: &str) {
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

    check_is_real_root_ok(&farm, path, false);
}

#[test_case("A/B/=b1")]
#[test_case("A/B/=a1")]
#[test_case("A/B/=C")]
#[cfg(not(target_family = "windows"))]
fn test_is_real_root_not_abs_symlinks_not_windows(path: &str) {
    let mut farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b1")
        .file("A/a1");

    farm.symlink_abs("A/B/=b1", "A/B/b1")
        .symlink_abs("A/B/=a1", "A/a1")
        .symlink_abs("A/B/=C", "A/C");

    check_is_real_root_ok(&farm, path, false);
}

mod helpers;
use helpers::*;
