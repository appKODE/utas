use assert_cmd::Command;
use assert_fs::{self};
use std::error::Error;

#[test]
fn case_android_1() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("utas")?;

    let temp = assert_fs::TempDir::new()?;

    let input = "tests/cases/android/output1";
    let output = temp.path();

    cmd.arg(input).arg(output.as_os_str());
    cmd.assert().success();
    assert!(file::dirs_contents_are_same(input, output)?);
    Ok(())
}
