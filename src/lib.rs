#![allow(dead_code)]
//! `fscan` — filesystem scanner and tool prober.
//!
//! ## Features
//! - `probe` *(default)*: detect tools by name via `which()` — UseCase 1
//! - `scan`: walk a directory and classify every file — UseCase 2
//! - `full`: both features enabled
//!
//! ## Quick start
//! ```rust
//! // UseCase 1: do we have a package manager?
//! use fscan::probe::{CommandProbe, ToolMeta, ProbeAction};
//! let p = CommandProbe::check(ToolMeta::new("dnf", "System", "Fedora/RHEL"));
//! match p.action() {
//!     ProbeAction::Use     => println!("run dnf"),
//!     ProbeAction::Fallback => println!("try something else"),
//!     ProbeAction::Install => println!("dnf not found"),
//! }
//!
//! // UseCase 2: what's in /usr/local/bin?
//! use fscan::scan::DirScanner;
//! let result = DirScanner::new("/usr/local/bin").scan().unwrap();
//! for entry in result.executables() {
//!     println!("{}", entry.path.display());
//! }
//! ```

pub mod fscan;
pub mod omni_command_executor;

