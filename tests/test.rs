use assert_cmd::Command;
use assert_fs::{self};
use file::{CompareDirsContentResult, Diff, DirDiff};
use std::{error::Error, path::Path};

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
    let result = file::compare_dirs_content(expected, output)?;
    match &result {
        CompareDirsContentResult::Eq => (),
        CompareDirsContentResult::Diffs(diffs) => eprintln!("{}", format_diffs(diffs)),
    }
    assert!(CompareDirsContentResult::Eq == result, "Dirs are different. See log above");
    Ok(())
}

fn format_result(result: CompareDirsContentResult) -> String {
    match result {
        CompareDirsContentResult::Eq => "OK".to_string(),
        CompareDirsContentResult::Diffs(diffs) => format_diffs(&diffs),
    }
}

fn format_diffs(diffs: &Vec<DirDiff>) -> String {
    let mut result = "".to_string();
    let mut index = 0;
    for diff in diffs {
        let item = match diff {
            DirDiff::Path { left, right } => format!(
                "{}. Paths are different: {} and {}\n___________________________________________________________\n\n",
                index,
                format_path(left),
                format_path(right)
            ),
            DirDiff::FileContent { path, diffs } => {
                format!(
                    "{}. In file {} diff content:\n {}___________________________________________________________\n\n",
                    index, 
                    path,
                    format_file_diffs(diffs),
                )
            }
        };
        index += 1;
        result.push_str(&item);
    }
    result
}

fn format_path(path: &Option<String>) -> String {
    match path {
        Some(path) => path.clone(),
        None => "|NO ANALOGUE|".to_string(),
    }
}

fn format_file_diffs(diffs: &Vec<Diff>) -> String {
    let mut result = "".to_string();
    for diff in diffs {
        result.push_str(&format!(
            "  Line {}.:\n{}\n{}",
            diff.line_number, diff.left, diff.right
        ));
    }

    result
}
