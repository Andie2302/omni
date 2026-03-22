use std::fs;
use std::path::Path;
use crate::error::OmniError;

pub fn create_directory<P: AsRef<Path>>(path: P) -> Result<(), OmniError> {
    let p = path.as_ref();

    if p.is_file() {
        return Err(OmniError::PathIsFile(p.to_path_buf()));
    }

    if !p.is_dir() {
        fs::create_dir_all(p).map_err(OmniError::CreateDirectory)?;
    }

    Ok(())
}

pub fn create_file<P: AsRef<Path>>(path: P) -> Result<(), OmniError> {
    let p = path.as_ref();

    if p.is_dir() {
        return Err(OmniError::PathIsDirectory(p.to_path_buf()));
    }

    if !p.is_file() {
        fs::File::create(p).map_err(OmniError::CreateFile)?;
    }

    Ok(())
}

pub fn create_file_and_folders<P: AsRef<Path>>(path: P) -> Result<(), OmniError> {
    let p = path.as_ref();

    if p.is_dir() {
        return Err(OmniError::PathIsDirectory(p.to_path_buf()));
    }

    if let Some(parent) = p.parent() {
        if parent.is_file() {
            return Err(OmniError::PathIsFile(parent.to_path_buf()));
        }
        create_directory(parent)?;
    }

    create_file(p)?;
    Ok(())
}
pub fn exists_directory<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_dir()
}

pub fn exists_file<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}