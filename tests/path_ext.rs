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

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/_b1", "A/B")]
#[test_case("A/B/_a1", "A")]
#[test_case("A/B/C/_a1", "A")]
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

    check_real_parent_ok(&farm, path, expected);
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

    check_real_parent_ok(&farm, path, expected);
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

    check_real_parent_ok(&farm, path, expected);
}

#[test_case("A/B/=b1", "A/B")]
#[test_case("A/B/=a1", "A")]
#[test_case("A/B/=C", "A")]
fn test_real_parent_abs_symlinks(path: &str, expected: &str) {
    let farm = LinkFarm::new();

    farm.dir("A")
        .dir("A/B")
        .dir("A/C")
        .file("A/B/b1")
        .file("A/a1")
        .symlink_abs("A/B/=b1", "A/B/b1")
        .symlink_abs("A/B/=a1", "A/a1")
        .symlink_abs("A/B/=C", "A/C");

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

    // since real_parent now returns io:Error, we can't distinguish different kinds of failures
    check_real_parent_err(&farm, path);
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
    check_real_parent_err(&farm, path);
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

mod helpers;
use helpers::*;
