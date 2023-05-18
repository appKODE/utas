use std::error::Error;

use assert_cmd::Command;
use predicates::prelude::predicate;

#[test]
fn print_args() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("utas")?;

    let input = "TEST_INPUT_DIR";
    let output = "TEST_OUTPUT_DIR";

    let expected_std = format!("{} {}\n", input, output);

    cmd.arg(input).arg(output);
    cmd.assert().success().stdout(predicate::eq(expected_std));

    Ok(())
}
