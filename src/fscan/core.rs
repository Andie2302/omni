//! Shared types — used by both `probe` (UseCase 1) and `scan` (UseCase 2).
#![allow(dead_code)]
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

// -------------------------------------------------------
// FileKind
// -------------------------------------------------------

/// Top-level classification of any filesystem entry.
/// `NonExecutable` stays intentionally flat for now — extend later
/// without breaking callers (e.g. `NonExecutable(MediaKind)`).
#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    Executable(ExecutableKind),
    NonExecutable,
    /// Could not be determined (permission denied, special device, …)
    Unknown(String),
}

impl FileKind {
    pub fn is_executable(&self) -> bool {
        matches!(self, FileKind::Executable(_))
    }
}

impl fmt::Display for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileKind::Executable(k)  => write!(f, "executable ({k})"),
            FileKind::NonExecutable  => write!(f, "non-executable"),
            FileKind::Unknown(r)     => write!(f, "unknown ({r})"),
        }
    }
}

// -------------------------------------------------------
// ExecutableKind
// -------------------------------------------------------

/// What kind of executable is this?
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutableKind {
    /// ELF / Mach-O / PE — no shebang, x-bit set.
    NativeBinary,
    /// Has a `#!` shebang line; we capture the interpreter.
    Script { interpreter: String },
    /// x-bit set but we couldn't read/parse it further.
    Unknown,
}

impl fmt::Display for ExecutableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutableKind::NativeBinary              => write!(f, "native binary"),
            ExecutableKind::Script { interpreter }    => write!(f, "script ({})", interpreter),
            ExecutableKind::Unknown                   => write!(f, "executable/unknown"),
        }
    }
}

// -------------------------------------------------------
// LinkKind  — how a path is represented on disk
// -------------------------------------------------------

/// Whether the resolved path is a direct file, a symlink (chain), or a wrapper.
#[derive(Debug, Clone)]
pub enum LinkKind {
    Direct,
    Symlink     { target: PathBuf },
    SymlinkChain { chain: Vec<PathBuf> },
}

impl LinkKind {
    /// Inspect `path` and return its link topology.
    pub fn detect(path: &Path) -> Self {
        let chain = Self::resolve_chain(path);
        match chain.len() {
            0 => LinkKind::Direct,
            1 => LinkKind::Symlink { target: chain.into_iter().next().unwrap() },
            _ => LinkKind::SymlinkChain { chain },
        }
    }

    /// Short inline annotation, e.g. "→ /usr/bin/python3.12"
    pub fn display_short(&self) -> String {
        match self {
            LinkKind::Direct                       => String::new(),
            LinkKind::Symlink { target }           => format!("→ {}", target.display()),
            LinkKind::SymlinkChain { chain }       => {
                let last = chain.last().unwrap();
                format!("→→ {} ({} hops)", last.display(), chain.len())
            }
        }
    }

    fn resolve_chain(path: &Path) -> Vec<PathBuf> {
        let mut chain = Vec::new();
        let mut current = path.to_path_buf();
        for _ in 0..10 {                     // guard against circular links
            match fs::read_link(&current) {
                Ok(target) => {
                    let resolved = if target.is_absolute() {
                        target
                    } else {
                        current.parent()
                            .unwrap_or(Path::new("/"))
                            .join(&target)
                    };
                    chain.push(resolved.clone());
                    current = resolved;
                }
                Err(_) => break,
            }
        }
        chain
    }
}

// -------------------------------------------------------
// classify_path  — shared logic, no `which` involved
// -------------------------------------------------------

/// Determine the `FileKind` for an already-known path.
/// Used by both `probe` (after `which` found it) and `scan` (during dir walk).
pub fn classify_path(path: &Path) -> FileKind {
    let meta = match fs::metadata(path) {
        Ok(m)  => m,
        Err(e) => return FileKind::Unknown(e.to_string()),
    };

    if !meta.is_file() {
        return FileKind::NonExecutable;   // dirs, sockets, devices
    }

    if is_executable_meta(&meta) {
        FileKind::Executable(detect_executable_kind(path))
    } else {
        FileKind::NonExecutable
    }
}

// -------------------------------------------------------
// Internal helpers
// -------------------------------------------------------

#[cfg(unix)]
fn is_executable_meta(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_meta(_meta: &fs::Metadata) -> bool {
    // On Windows every .exe/.bat counts — extend if needed
    false
}

fn detect_executable_kind(path: &Path) -> ExecutableKind {
    let file = match fs::File::open(path) {
        Ok(f)  => f,
        Err(_) => return ExecutableKind::Unknown,
    };
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();

    if reader.read_line(&mut first_line).is_err() || first_line.is_empty() {
        return ExecutableKind::Unknown;
    }

    if let Some(interp) = first_line.strip_prefix("#!") {
        // "#!/usr/bin/env python3" → last token is "python3"
        // "#!/bin/bash"           → last token is "/bin/bash", basename = "bash"
        let raw = interp.split_whitespace().last().unwrap_or("unknown");
        let interpreter = Path::new(raw)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(raw)
            .to_string();
        ExecutableKind::Script { interpreter }
    } else {
        ExecutableKind::NativeBinary
    }
}
