use anyhow::Result;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

/// A struct to represent a file in the request message
///
/// # Attributes
///
/// * `path` - The path of the file
/// * `file_name` - The name of the file
/// * `content` - The content of the file
#[derive(Deserialize, Serialize, Debug)]
pub struct FileModel {
    pub path: PathBuf,
    pub file_name: String,
    pub content: String,
}

impl FileModel {
    pub fn new(path: PathBuf, file_name: String, content: String) -> Self {
        Self {
            path,
            file_name,
            content,
        }
    }
}

/// A struct to represent the result of a command
///
/// # Attributes
///
/// * `stdout` - The stdout of the command
/// * `stderr` - The stderr of the command
/// * `exit_code` - The exit code of the command
#[derive(Deserialize, Serialize, Debug)]
pub struct CodeReturn {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CodeReturn {
    pub fn new(stdout: String, stderr: String, exit_code: i32) -> Self {
        Self {
            stdout,
            stderr,
            exit_code,
        }
    }
}

/// A trait responsible for CRU operations on files and dir
/// Mainly used in tests to mock file operations
trait FileHandler: Read + Write {
    fn create<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        Self: Sized;
    fn read<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        Self: Sized;
}

impl FileHandler for File {
    fn create<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        Self: Sized,
    {
        let file = File::create(path)?;
        Ok(file)
    }

    fn read<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        Self: Sized,
    {
        let file = File::open(path)?;
        Ok(file)
    }
}
