//! UseCase 2 — **Directory scanning by path**.
//!
//! You have a directory path and want to know what's in it.
//! No `which` needed — we walk the tree and classify each entry.
//!
//! # Example
//! ```rust
//! use fscan::scan::DirScanner;
//!
//! let results = DirScanner::new("/usr/bin").scan().unwrap();
//! for entry in results.executables() {
//!     println!("{}", entry.path.display());
//! }
//! ```
#![allow(dead_code)]
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::fscan::core::{classify_path, FileKind, LinkKind};
// -------------------------------------------------------
// ScanEntry — one classified file found during the walk
// -------------------------------------------------------

#[derive(Debug)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub kind: FileKind,
    pub link: LinkKind,
}

impl ScanEntry {
    pub fn is_executable(&self) -> bool {
        self.kind.is_executable()
    }
}

// -------------------------------------------------------
// ScanResult — the full result of a directory scan
// -------------------------------------------------------

pub struct ScanResult {
    pub root:    PathBuf,
    pub entries: Vec<ScanEntry>,
}

impl ScanResult {
    pub fn executables(&self) -> impl Iterator<Item = &ScanEntry> {
        self.entries.iter().filter(|e| e.is_executable())
    }

    pub fn non_executables(&self) -> impl Iterator<Item = &ScanEntry> {
        self.entries.iter().filter(|e| !e.is_executable())
    }

    pub fn count_executable(&self) -> usize {
        self.executables().count()
    }

    pub fn count_non_executable(&self) -> usize {
        self.non_executables().count()
    }
}

// -------------------------------------------------------
// DirScanner — builder-style config + scan execution
// -------------------------------------------------------

pub struct DirScanner {
    root:          PathBuf,
    max_depth:     Option<usize>,
    follow_links:  bool,
}

impl DirScanner {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root:         root.as_ref().to_path_buf(),
            max_depth:    None,
            follow_links: false,
        }
    }

    /// Limit recursion depth (1 = only direct children).
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Follow symbolic links during traversal.
    pub fn follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    /// Run the scan and return all classified entries.
    pub fn scan(self) -> Result<ScanResult, std::io::Error> {
        if !self.root.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("path does not exist: {}", self.root.display()),
            ));
        }

        let mut walker = WalkDir::new(&self.root)
            .follow_links(self.follow_links);

        if let Some(depth) = self.max_depth {
            walker = walker.max_depth(depth);
        }

        let mut entries = Vec::new();

        for entry in walker {
            let entry = match entry {
                Ok(e)  => e,
                Err(_) => continue,   // permission denied etc. — skip silently
            };

            // Skip the root directory itself and all subdirectories
            if entry.file_type().is_dir() {
                continue;
            }

            let path = entry.path().to_path_buf();
            let kind = classify_path(&path);
            let link = LinkKind::detect(&path);

            entries.push(ScanEntry { path, kind, link });
        }

        Ok(ScanResult { root: self.root, entries })
    }
}

// -------------------------------------------------------
// Report formatting
// -------------------------------------------------------

pub fn print_scan_report(result: &ScanResult) {
    println!("\n  Scan: {}", result.root.display());
    println!("{:=<70}", "");
    println!(
        "  {} Dateien total  |  {} ausführbar  |  {} nicht ausführbar",
        result.entries.len(),
        result.count_executable(),
        result.count_non_executable(),
    );
    println!("{:-<70}", "");

    if result.count_executable() > 0 {
        println!("\n  [Ausführbar]");
        for e in result.executables() {
            let link = e.link.display_short();
            let suffix = if link.is_empty() { String::new() } else { format!("  {link}") };
            println!("  ✔  {}  ({}){}", e.path.display(), e.kind, suffix);
        }
    }

    if result.count_non_executable() > 0 {
        println!("\n  [Nicht ausführbar]");
        for e in result.non_executables() {
            println!("  ·  {}", e.path.display());
        }
    }
    println!();
}
