use assert_cmd::Command;
use assert_fs::{self};
use std::error::Error;

#[test]
fn case_android_1() -> Result<(), Box<dyn Error>> {
    basic_test_case("case1")
}

#[test]
fn case_android_2() -> Result<(), Box<dyn Error>> {
    basic_test_case("case2")
}

#[test]
fn case_android_3() -> Result<(), Box<dyn Error>> {
    basic_test_case("case3")
}

#[test]
fn case_android_4() -> Result<(), Box<dyn Error>> {
    basic_test_case("case4")
}

#[test]
fn case_android_5() -> Result<(), Box<dyn Error>> {
    basic_test_case("case5")
}

#[test]
fn case_android_6() -> Result<(), Box<dyn Error>> {
    basic_test_case("case6")
}

#[test]
fn case_android_7() -> Result<(), Box<dyn Error>> {
    basic_test_case("case7")
}

#[test]
fn case_android_8() -> Result<(), Box<dyn Error>> {
    basic_test_case("case8")
}

fn basic_test_case(case_rel_path: &str) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("utas")?;

    let temp = assert_fs::TempDir::new()?;

    let input = format!("tests/cases/android/{}/input", case_rel_path);
    let output = temp.path();
    let expected = format!("tests/cases/android/{}/output", case_rel_path);

    cmd.arg(&input).arg(output.as_os_str());
    cmd.assert().success();
    assert!(file::dirs_contents_are_same(expected, output)?);
    Ok(())
}
