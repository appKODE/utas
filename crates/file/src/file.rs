use anyhow::{Ok, Result};
use assert_fs::fixture::FileWriteStr;
use queues::{queue, IsQueue, Queue};
use std::cmp::max;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(PartialEq, Eq, Debug)]
pub enum CompareContentResult {
    Eq,
    Diffs(Vec<Diff>),
}

#[derive(PartialEq, Eq, Debug)]
pub struct Diff {
    pub line_number: u32,
    pub left: String,
    pub right: String,
}

#[derive(PartialEq, Eq, Debug)]
pub enum CompareDirsContentResult {
    Eq,
    Diffs(Vec<DirDiff>),
}

#[derive(PartialEq, Eq, Debug)]
pub enum DirDiff {
    Path {
        left: Option<String>,
        right: Option<String>,
    },
    FileContent {
        path: String,
        diffs: Vec<Diff>,
    },
}

pub fn copy_recursively(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&destination)?;
    for item in fs::read_dir(source)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            copy_recursively(item.path(), destination.as_ref().join(item.file_name()))?;
        } else {
            fs::copy(item.path(), destination.as_ref().join(item.file_name()))?;
        }
    }
    Ok(())
}

/// Check if all files in dirs have the same content and paths to files with contents
pub fn dirs_contents_are_same(dir1: impl AsRef<Path>, dir2: impl AsRef<Path>) -> Result<bool> {
    let mut paths1 = get_all_file_paths(&dir1.as_ref())?;
    let mut paths2 = get_all_file_paths(&dir2.as_ref())?;
    if paths1.len() != paths2.len() {
        return Ok(false);
    }
    paths1.sort();
    paths2.sort();
    for i in 0..paths1.len() {
        let path1 = &paths1[i].strip_prefix(&dir1.as_ref());
        let path2 = &paths2[i].strip_prefix(&dir2.as_ref());
        if path1 != path2 || !files_are_same(&paths1[i], &paths2[i])? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn get_all_file_paths(dir: &Path) -> Result<Vec<Box<Path>>> {
    let mut paths: Vec<Box<Path>> = vec![];
    let mut dirs = queue![Box::from(dir)];
    loop {
        let dir = dirs.remove().unwrap();
        for item in fs::read_dir(&dir)? {
            let item = item?;
            if item.file_type()?.is_dir() {
                dirs.add(Box::from(dir.join(item.file_name()).as_ref()))
                    .unwrap();
            } else {
                // TODO move it to appropriate place (in file.rs shouldn't be any platform related concrecity)
                // https://github.com/appKODE/utas/issues/33
                // Skipping macOS system file
                if item.file_name() != ".DS_Store" {
                    paths.push(Box::from(item.path().as_path()))
                }
            }
        }
        if dirs.size() == 0 {
            break;
        }
    }
    Ok(paths)
}

pub fn files_are_same(file1: impl AsRef<Path>, file2: impl AsRef<Path>) -> Result<bool> {
    let file1 = File::open(file1)?;
    let file2 = File::open(file2)?;
    if file1.metadata()?.len() != file2.metadata()?.len() {
        return Ok(false);
    }

    let file1 = BufReader::new(file1);
    let file2 = BufReader::new(file2);

    for (bytes1, bytes2) in file1.bytes().zip(file2.bytes()) {
        if bytes1? != bytes2? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Compare all files content and paths in dirs
pub fn compare_dirs_content(
    dir1: impl AsRef<Path>,
    dir2: impl AsRef<Path>,
) -> Result<CompareDirsContentResult> {
    let mut paths1 = get_all_file_paths(&dir1.as_ref())?;
    let mut paths2 = get_all_file_paths(&dir2.as_ref())?;

    paths1.sort();
    paths2.sort();

    let mut diffs: Vec<DirDiff> = vec![];

    for i in 0..max(paths1.len(), paths2.len()) {
        let path1 = if i < paths1.len() {
            Option::Some(paths1[i].strip_prefix(&dir1.as_ref())?)
        } else {
            Option::None
        };
        let path2 = if i < paths2.len() {
            Option::Some(paths2[i].strip_prefix(&dir2.as_ref())?)
        } else {
            Option::None
        };

        if path1.is_none() {
            diffs.push(DirDiff::Path {
                left: Option::None,
                right: Option::Some(format!("{:?}", paths2[i])),
            });
            continue;
        }

        if path2.is_none() {
            diffs.push(DirDiff::Path {
                left: Option::Some(format!("{:?}", paths1[i])),
                right: Option::None,
            });
            continue;
        }

        if path1 != path2 {
            diffs.push(DirDiff::Path {
                left: Option::Some(format!("{:?}", paths1[i])),
                right: Option::Some(format!("{:?}", paths2[i])),
            });
            continue;
        }

        match compare_files_content(&paths1[i], &paths2[i])? {
            CompareContentResult::Eq => continue,
            CompareContentResult::Diffs(file_diffs) => diffs.push(DirDiff::FileContent {
                path: format!("{:?}", paths1[i]),
                diffs: file_diffs,
            }),
        }
    }

    let result = if diffs.is_empty() {
        CompareDirsContentResult::Eq
    } else {
        CompareDirsContentResult::Diffs(diffs)
    };

    Ok(result)
}

pub fn compare_files_content(
    file1: impl AsRef<Path>,
    file2: impl AsRef<Path>,
) -> Result<CompareContentResult> {
    let file1 = File::open(file1)?;
    let file2 = File::open(file2)?;

    let mut file1 = BufReader::new(file1);
    let mut file2 = BufReader::new(file2);

    let mut line1 = "".to_string();
    let mut line2 = "".to_string();

    let mut diffs: Vec<Diff> = vec![];
    let mut line_number = 1;

    loop {
        let bytes1 = file1.read_line(&mut line1)?;
        let bytes2 = file2.read_line(&mut line2)?;

        // read_line does't handle \r\n if we read file on windows 
        line1 = line1.trim().to_string();
        line2 = line2.trim().to_string();

        if line1 != line2 {
            diffs.push(Diff {
                line_number: line_number,
                left: line1.clone(),
                right: line2.clone(),
            })
        }

        line_number += 1;

        if bytes1 == 0 && bytes2 == 0 {
            break;
        }

        line1 = "".to_string();
        line2 = "".to_string();
    }

    let result = if diffs.is_empty() {
        CompareContentResult::Eq
    } else {
        CompareContentResult::Diffs(diffs)
    };

    Ok(result)
}

#[test]
fn files_are_same_0() -> Result<()> {
    let file1 = assert_fs::NamedTempFile::new("file1.txt")?;
    file1.write_str("lol\nkek\nchebureck\nlolkek")?;
    let file2 = assert_fs::NamedTempFile::new("file2.txt")?;
    file2.write_str("lol\nkek\nchebureck\nlolkek")?;
    assert!(files_are_same(file1.as_ref(), file2.as_ref())?);
    Ok(())
}

#[test]
fn files_are_same_1() -> Result<()> {
    let dir = assert_fs::TempDir::new()?;
    let file1 = assert_fs::NamedTempFile::new(dir.as_ref().join("file1.txt"))?;
    file1.write_str("kek")?;
    let file2 = assert_fs::NamedTempFile::new(dir.as_ref().join("file2.txt"))?;
    file2.write_str("kek")?;
    assert!(files_are_same(file1.as_ref(), file2.as_ref())?);
    Ok(())
}

#[test]
fn files_are_not_same() -> Result<()> {
    let file1 = assert_fs::NamedTempFile::new("file1.txt")?;
    file1.write_str("1000-2000")?;
    let file2 = assert_fs::NamedTempFile::new("file2.txt")?;
    file2.write_str("1000-3_000")?;
    assert!(!files_are_same(file1.as_ref(), file2.as_ref())?);
    Ok(())
}

#[test]
fn dir_is_equivalent_to_itself() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    assert!(dirs_contents_are_same(dir1.as_ref(), dir1.as_ref())?);
    Ok(())
}

#[test]
fn dirs_are_same_0() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("file2.txt"))?;
    dir2_file2.write_str("FILE2_CONTENT")?;

    assert!(dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn dirs_are_same_2_level_nesting() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2").join("file2.txt"))?;
    dir2_file2.write_str("FILE2_CONTENT")?;

    assert!(dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn empty_dirs_are_same() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir2 = assert_fs::TempDir::new()?;
    assert!(dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn dirs_are_not_same_when_one_of_them_is_empty() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    assert!(!dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn dirs_are_not_same_when_paths_are_different() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path_3").join("file2.txt"))?;
    dir2_file2.write_str("FILE2_CONTENT")?;

    assert!(!dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn dirs_are_not_same_when_contents_are_different() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1/file1.txt"))?;
    dir1_file1.write_str("FILE_2_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2/file2.txt"))?;
    dir1_file2.write_str("FILE_1_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE_1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2").join("file2.txt"))?;
    dir2_file2.write_str("FILE_2_CONTENT")?;

    assert!(!dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn files_have_eq_content() -> Result<()> {
    let file1 = assert_fs::NamedTempFile::new("file1.txt")?;
    file1.write_str("lol\nkek\nchebureck\nlolkek")?;
    let file2 = assert_fs::NamedTempFile::new("file2.txt")?;
    file2.write_str("lol\nkek\nchebureck\nlolkek")?;
    let result = compare_files_content(file1, file2)?;

    assert_eq!(CompareContentResult::Eq, result);
    Ok(())
}

#[test]
fn files_have_diff_content_in_1_lines() -> Result<()> {
    let file1 = assert_fs::NamedTempFile::new("file1.txt")?;
    file1.write_str("lol\nkek\nchebureck\nlolkek")?;
    let file2 = assert_fs::NamedTempFile::new("file2.txt")?;
    file2.write_str("lol\nkek\nWAAAAAA\nlolkek")?;
    let result = compare_files_content(file1, file2)?;

    let expected = CompareContentResult::Diffs(vec![Diff {
        line_number: 3,
        left: "chebureck".to_string(),
        right: "WAAAAAA".to_string(),
    }]);
    assert_eq!(expected, result);
    Ok(())
}

#[test]
fn files_have_diff_content_in_2_lines() -> Result<()> {
    let file1 = assert_fs::NamedTempFile::new("file1.txt")?;
    file1.write_str("lol\nkek\nchebureck\nlolkek")?;
    let file2 = assert_fs::NamedTempFile::new("file2.txt")?;
    file2.write_str("lol\nkek\nWAAAAAA\nlolkekus")?;
    let result = compare_files_content(file1, file2)?;

    let expected = CompareContentResult::Diffs(vec![
        Diff {
            line_number: 3,
            left: "chebureck".to_string(),
            right: "WAAAAAA".to_string(),
        },
        Diff {
            line_number: 4,
            left: "lolkek".to_string(),
            right: "lolkekus".to_string(),
        },
    ]);
    assert_eq!(expected, result);
    Ok(())
}

#[test]
fn files_have_diff_content_length() -> Result<()> {
    let file1 = assert_fs::NamedTempFile::new("file1.txt")?;
    file1.write_str("lol\nkek\nchebureck\nlolkek")?;
    let file2 = assert_fs::NamedTempFile::new("file2.txt")?;
    file2.write_str("lol\nkek\nchebureck\n")?;
    let result = compare_files_content(file1, file2)?;

    let expected = CompareContentResult::Diffs(vec![Diff {
        line_number: 4,
        left: "lolkek".to_string(),
        right: "".to_string(),
    }]);
    assert_eq!(expected, result);
    Ok(())
}

#[test]
fn dirs_content_is_equivalent_to_itself() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;
    let dir2 = assert_fs::TempDir::new()?;

    let result = compare_dirs_content(dir1.as_ref(), dir1.as_ref())?;

    assert_eq!(CompareDirsContentResult::Eq, result);
    Ok(())
}

#[test]
fn dirs_have_the_same_content() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("file2.txt"))?;
    dir2_file2.write_str("FILE2_CONTENT")?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;

    assert_eq!(CompareDirsContentResult::Eq, result);
    Ok(())
}

#[test]
fn dirs_have_the_same_content_2_level() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2").join("file2.txt"))?;
    dir2_file2.write_str("FILE2_CONTENT")?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;

    assert_eq!(CompareDirsContentResult::Eq, result);
    Ok(())
}

#[test]
fn empty_dirs_have_the_same_content() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir2 = assert_fs::TempDir::new()?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;
    assert_eq!(CompareDirsContentResult::Eq, result);
    Ok(())
}

#[test]
fn dirs_have_diff_content_if_one_of_then_is_empty() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;
    let dir2 = assert_fs::TempDir::new()?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;
    let expected = CompareDirsContentResult::Diffs(vec![
        DirDiff::Path {
            left: Option::Some(format!("{:?}", dir1_file1.path())),
            right: None,
        },
        DirDiff::Path {
            left: Option::Some(format!("{:?}", dir1_file2.path())),
            right: None,
        },
    ]);

    assert_eq!(expected, result);
    Ok(())
}

#[test]
fn dirs_have_diff_content_if_files_have_different_paths() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path_3").join("file2.txt"))?;
    dir2_file2.write_str("FILE2_CONTENT")?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;
    let expected = CompareDirsContentResult::Diffs(vec![DirDiff::Path {
        left: Option::Some(format!("{:?}", dir1_file2.path())),
        right: Option::Some(format!("{:?}", dir2_file2.path())),
    }]);

    assert_eq!(expected, result);
    Ok(())
}

#[test]
fn dirs_have_diff_content_if_files_have_different_content() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2").join("file2.txt"))?;
    dir2_file2.write_str("FIRST_LINE\nSECOND_LINE")?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;
    let expected = CompareDirsContentResult::Diffs(vec![DirDiff::FileContent {
        path: format!("{:?}", dir1_file2.path()),
        diffs: vec![
            Diff {
                line_number: 1,
                left: "FILE2_CONTENT".to_string(),
                right: "FIRST_LINE".to_string(),
            },
            Diff {
                line_number: 2,
                left: "".to_string(),
                right: "SECOND_LINE".to_string(),
            },
        ],
    }]);

    assert_eq!(expected, result);
    Ok(())
}

#[test]
fn dirs_have_diff_content_if_files_have_different_content_and_path() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1").join("file!1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2").join("file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1").join("file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2").join("file2.txt"))?;
    dir2_file2.write_str("FIRST_LINE\n\nSECOND_LINE")?;

    let result = compare_dirs_content(dir1.as_ref(), dir2.as_ref())?;
    let expected = CompareDirsContentResult::Diffs(vec![
        DirDiff::Path {
            left: Option::Some(format!("{:?}", dir1_file1.path())),
            right: Option::Some(format!("{:?}", dir2_file1.path())),
        },
        DirDiff::FileContent {
            path: format!("{:?}", dir1_file2.path()),
            diffs: vec![
                Diff {
                    line_number: 1,
                    left: "FILE2_CONTENT".to_string(),
                    right: "FIRST_LINE".to_string(),
                },
                Diff {
                    line_number: 3,
                    left: "".to_string(),
                    right: "SECOND_LINE".to_string(),
                },
            ],
        },
    ]);

    assert_eq!(expected, result);
    Ok(())
}
