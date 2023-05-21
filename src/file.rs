use anyhow::{Ok, Result};
use assert_fs::prelude::FileWriteStr;
use queues::{queue, IsQueue, Queue};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;

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
    let paths1 = get_all_file_paths(&dir1.as_ref())?;
    let paths2 = get_all_file_paths(&dir2.as_ref())?;
    if paths1.len() != paths2.len() {
        return Ok(false);
    }
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
                paths.push(Box::from(item.path().as_path()))
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
    let file1 = assert_fs::NamedTempFile::new("dir/file1.txt")?;
    file1.write_str("kek")?;
    let file2 = assert_fs::NamedTempFile::new("dir/file2.txt")?;
    file2.write_str("kek")?;
    assert!(files_are_same(file1.as_ref(), file2.as_ref())?);
    Ok(())
}

#[test]
fn files_ara_not_same() -> Result<()> {
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
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1/file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2/file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1/file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2/file2.txt"))?;
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
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1/file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2/file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    assert!(!dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}

#[test]
fn dirs_are_not_same_when_paths_are_different() -> Result<()> {
    let dir1 = assert_fs::TempDir::new()?;
    let dir1_file1 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path1/file1.txt"))?;
    dir1_file1.write_str("FILE1_CONTENT")?;
    let dir1_file2 = assert_fs::NamedTempFile::new(dir1.as_ref().join("path2/file2.txt"))?;
    dir1_file2.write_str("FILE2_CONTENT")?;

    let dir2 = assert_fs::TempDir::new()?;
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1/file1.txt"))?;
    dir2_file1.write_str("FILE1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path_3/file2.txt"))?;
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
    let dir2_file1 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path1/file1.txt"))?;
    dir2_file1.write_str("FILE_1_CONTENT")?;
    let dir2_file2 = assert_fs::NamedTempFile::new(dir2.as_ref().join("path2/file2.txt"))?;
    dir2_file2.write_str("FILE_2_CONTENT")?;

    assert!(!dirs_contents_are_same(dir1.as_ref(), dir2.as_ref())?);
    Ok(())
}
