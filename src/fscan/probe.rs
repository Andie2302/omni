//! UseCase 1 — **Tool-Probing by name**.
//!
//! You know the name ("dnf", "cargo") but not whether it exists.
//! `which()` searches the PATH; we classify what we find.
//!
//! # Example
//! ```rust
//! use fscan::probe::{ToolMeta, CommandProbe, print_probe_report};
//!
//! let tools = vec![
//!     ToolMeta::new("dnf",   "System",   "Fedora/RHEL"),
//!     ToolMeta::new("cargo", "Language", "Rust"),
//! ];
//! let results: Vec<CommandProbe> = tools.into_iter().map(CommandProbe::check).collect();
//! print_probe_report(&results);
//! ```
#![allow(dead_code)]
use std::path::PathBuf;
use which::which;

use crate::fscan::core::{classify_path, ExecutableKind, FileKind, LinkKind};

// -------------------------------------------------------
// ToolMeta — static description of a tool we want to find
// -------------------------------------------------------

/// Static metadata about a tool we're looking for.
/// All fields are `&'static str` — known at compile time.
#[derive(Debug, Clone)]
pub struct ToolMeta {
    pub name:        &'static str,
    pub category:    &'static str,
    pub description: &'static str,
}

impl ToolMeta {
    pub fn new(name: &'static str, category: &'static str, description: &'static str) -> Self {
        Self { name, category, description }
    }
}

// -------------------------------------------------------
// ProbeStatus — what we found (or didn't)
// -------------------------------------------------------

#[derive(Debug)]
pub enum ProbeStatus {
    /// Tool found at this path with this kind.
    Found {
        path: PathBuf,
        kind: FileKind,
        link: LinkKind,
    },
    /// `which` returned nothing — not in PATH.
    Missing,
}

impl ProbeStatus {
    pub fn is_found(&self) -> bool {
        matches!(self, ProbeStatus::Found { .. })
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            ProbeStatus::Found { path, .. } => Some(path),
            ProbeStatus::Missing            => None,
        }
    }

    /// Convenience: true when found AND executable (should always be true, but
    /// guards against weird PATH entries like data files named "python").
    pub fn is_usable(&self) -> bool {
        matches!(self, ProbeStatus::Found { kind: FileKind::Executable(_), .. })
    }
}

// -------------------------------------------------------
// CommandProbe — result of probing a single tool
// -------------------------------------------------------

#[derive(Debug)]
pub struct CommandProbe {
    pub meta:   ToolMeta,
    pub status: ProbeStatus,
}

impl CommandProbe {
    /// Probe by name: search PATH with `which`, then classify what we find.
    pub fn check(meta: ToolMeta) -> Self {
        let status = match which(meta.name) {
            Ok(path) => {
                let link = LinkKind::detect(&path);
                let kind = classify_path(&path);
                ProbeStatus::Found { path, kind, link }
            }
            Err(_) => ProbeStatus::Missing,
        };
        Self { meta, status }
    }

    /// Quick check: is this tool available and usable right now?
    pub fn is_available(&self) -> bool {
        self.status.is_usable()
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.status.path()
    }

    /// Decide what to do: use it, fall back, or install.
    pub fn action(&self) -> ProbeAction {
        match &self.status {
            ProbeStatus::Found { kind: FileKind::Executable(_), .. } => ProbeAction::Use,
            ProbeStatus::Found { .. }  => ProbeAction::Fallback,   // found but not executable
            ProbeStatus::Missing       => ProbeAction::Install,
        }
    }
}

/// What the caller should do based on the probe result.
#[derive(Debug, PartialEq)]
pub enum ProbeAction {
    Use,
    Fallback,
    Install,
}

// -------------------------------------------------------
// Report formatting
// -------------------------------------------------------

pub fn print_probe_report(probes: &[CommandProbe]) {
    // Collect categories in insertion order
    let mut categories: Vec<&str> = Vec::new();
    for p in probes {
        if !categories.contains(&p.meta.category) {
            categories.push(p.meta.category);
        }
    }

    let (cn, cs, cp, cd) = (16usize, 2usize, 44usize, 28usize);
    let total = cn + cs + 1 + cp + 2 + cd + 6;

    for cat in categories {
        println!("\n  [{cat}]");
        println!("{:=<total$}", "");
        println!("{:<cn$}  {:<cs$} {:<cp$}  {:<cd$}", "Tool", "", "Pfad / Link", "Beschreibung");
        println!("{:-<total$}", "");

        for p in probes.iter().filter(|p| p.meta.category == cat) {
            let icon = if p.is_available() { "✔" } else { "✘" };

            let path_col = match &p.status {
                ProbeStatus::Found { path, link, .. } => {
                    let link_str = link.display_short();
                    if link_str.is_empty() {
                        path.display().to_string()
                    } else {
                        format!("{} {}", path.display(), link_str)
                    }
                }
                ProbeStatus::Missing => "---".to_string(),
            };

            let kind_note = match &p.status {
                ProbeStatus::Found { kind: FileKind::Executable(ExecutableKind::Script { interpreter }), .. } => {
                    format!("[{}] {}", interpreter, p.meta.description)
                }
                _ => p.meta.description.to_string(),
            };

            println!("{:<cn$}  {:<cs$} {:<cp$}  {:<cd$}", p.meta.name, icon, path_col, kind_note);
        }
    }
    println!();
}

// -------------------------------------------------------
// Package-manager registry (lives here, not in core)
// -------------------------------------------------------

pub mod registry {
    use super::ToolMeta;

    pub fn default_package_managers() -> Vec<ToolMeta> {
        vec![
            // System
            ToolMeta::new("dnf",         "System",    "Fedora/RHEL modern"),
            ToolMeta::new("dnf5",        "System",    "Fedora/RHEL next-gen"),
            ToolMeta::new("rpm-ostree",  "System",    "Bazzite/Silverblue"),
            ToolMeta::new("apt",         "System",    "Debian/Ubuntu"),
            ToolMeta::new("pacman",      "System",    "Arch Linux"),
            ToolMeta::new("zypper",      "System",    "openSUSE"),
            ToolMeta::new("emerge",      "System",    "Gentoo"),
            ToolMeta::new("xbps-install","System",    "Void Linux"),
            ToolMeta::new("apk",         "System",    "Alpine Linux"),
            ToolMeta::new("nix",         "System",    "Nix (flakes)"),
            // Universal
            ToolMeta::new("flatpak",     "Universal", "Flatpak apps"),
            ToolMeta::new("snap",        "Universal", "Snap apps"),
            ToolMeta::new("brew",        "Universal", "Homebrew"),
            ToolMeta::new("distrobox",   "Universal", "Container environments"),
            // Language
            ToolMeta::new("cargo",       "Language",  "Rust"),
            ToolMeta::new("pip3",        "Language",  "Python 3"),
            ToolMeta::new("uv",          "Language",  "Python fast (Astral)"),
            ToolMeta::new("npm",         "Language",  "Node.js"),
            ToolMeta::new("bun",         "Language",  "Bun runtime"),
            ToolMeta::new("gem",         "Language",  "Ruby"),
            ToolMeta::new("go",          "Language",  "Go"),
        ]
    }
}
