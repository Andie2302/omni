use std::fs;
use std::path::Path;
use crate::error::OmniError;


pub fn create_directory<P: AsRef<Path>>(path: P) -> Result<(), OmniError> {
    let p = path.as_ref();
    if !p.exists() {
        fs::create_dir_all(p).map_err(OmniError::CreateDirectory)?;
    }
    Ok(())
}

pub fn create_file<P: AsRef<Path>>(path: P) -> Result<(), OmniError> {
    let p = path.as_ref();
    if !p.exists() {
        fs::File::create(p).map_err(OmniError::CreateFile)?;
    }
    Ok(())
}

pub fn create_file_and_folders<P: AsRef<Path>>(path: P) -> Result<(), OmniError> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        create_directory(parent)?;
    }
    create_file(p)?;
    Ok(())
}
