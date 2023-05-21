use anyhow::{Ok, Result};
use std::fs;
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
