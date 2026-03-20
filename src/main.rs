use crate::error::OmniError;

mod utils;
mod error;

fn main() -> Result<(), OmniError> {
    let pfad = "data/config.json";
    utils::create_file_and_folders(pfad)?;
    println!("✅ Datei und Verzeichnisse für '{}' wurden vorbereitet.", pfad);
    Ok(())
}