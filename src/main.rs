mod tools;

use which::which;
use std::path::PathBuf;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader};

// -------------------------------------------------------
// Kern-Datentypen
// -------------------------------------------------------

/// Wie ein Tool im Dateisystem vorliegt
#[derive(Debug)]
pub enum LinkKind {
    /// Direkte Binärdatei, kein Symlink
    Direct,
    /// Einfacher Symlink (ein Hop)
    Symlink { target: PathBuf },
    /// Mehrfach verkettete Symlinks (z.B. java -> alternatives -> jvm/bin/java)
    SymlinkChain { chain: Vec<PathBuf> },
    /// Shell-Wrapper-Skript (shebang erkennbar)
    ShellWrapper { interpreter: String },
}

impl LinkKind {
    /// Analysiert einen gefundenen Pfad vollständig
    pub fn detect(path: &PathBuf) -> Self {
        let chain = Self::resolve_chain(path);

        match chain.len() {
            // Kein Symlink
            0 => {
                // Trotzdem prüfen ob Shell-Wrapper
                Self::detect_shell_wrapper(path)
                    .unwrap_or(LinkKind::Direct)
            }
            // Genau ein Hop
            1 => LinkKind::Symlink { target: chain.into_iter().next().unwrap() },
            // Mehrere Hops
            _ => {
                // Letztes Ziel auf Shell-Wrapper prüfen
                let final_target = chain.last().unwrap().clone();
                if let Some(LinkKind::ShellWrapper { interpreter }) =
                    Self::detect_shell_wrapper(&final_target)
                {
                    return LinkKind::ShellWrapper { interpreter };
                }
                LinkKind::SymlinkChain { chain }
            }
        }
    }

    /// Verfolgt die komplette Symlink-Kette (ohne den Startpfad selbst)
    fn resolve_chain(path: &PathBuf) -> Vec<PathBuf> {
        let mut chain = Vec::new();
        let mut current = path.clone();
        let max_hops = 10; // Schutz vor zirkulären Links

        for _ in 0..max_hops {
            match fs::read_link(&current) {
                Ok(target) => {
                    // Relative Symlinks auflösen
                    let resolved = if target.is_absolute() {
                        target
                    } else {
                        current.parent()
                            .unwrap_or(std::path::Path::new("/"))
                            .join(&target)
                    };
                    chain.push(resolved.clone());
                    current = resolved;
                }
                Err(_) => break, // Kein weiterer Symlink → Ende der Kette
            }
        }
        chain
    }

    /// Liest die erste Zeile einer Datei und erkennt Shebangs
    fn detect_shell_wrapper(path: &PathBuf) -> Option<LinkKind> {
        let file = fs::File::open(path).ok()?;
        let mut reader = BufReader::new(file);
        let mut first_line = String::new();
        reader.read_line(&mut first_line).ok()?;

        if first_line.starts_with("#!") {
            // "#/usr/bin/env python3" → "python3"
            // "#!/bin/bash"          → "bash"
            let interpreter = first_line
                .trim_start_matches("#!")
                .split_whitespace()
                .last() // letztes Token = tatsächlicher Interpreter bei `env`
                .unwrap_or("unknown")
                .to_string();
            Some(LinkKind::ShellWrapper { interpreter })
        } else {
            None
        }
    }

    /// Kompakte einzeilige Darstellung für die Tabelle
    pub fn display_short(&self) -> String {
        match self {
            LinkKind::Direct => String::new(),
            LinkKind::Symlink { target } =>
                format!("→ {}", target.display()),
            LinkKind::SymlinkChain { chain } =>
                format!("→→ {} ({})", chain.last().unwrap().display(), chain.len()),
            LinkKind::ShellWrapper { interpreter } =>
                format!("[wrapper: {}]", interpreter),
        }
    }

    /// Icon für die Status-Spalte
    pub fn icon(&self) -> &'static str {
        match self {
            LinkKind::Direct => "✔",
            LinkKind::Symlink { .. } => "⤷",
            LinkKind::SymlinkChain { .. } => "⤷⤷",
            LinkKind::ShellWrapper { .. } => "📜",
        }
    }
}

/// Ob ein Tool gefunden wurde – mit strukturiertem Pfad oder Fehlergrund
#[derive(Debug)]
pub enum AuditStatus {
    Found {
        path: PathBuf,
        link: LinkKind,
    },
    Missing,
    // Erweiterbar: z.B. Forbidden(String), WrongVersion(String)
}

impl AuditStatus {
    pub fn is_found(&self) -> bool {
        matches!(self, AuditStatus::Found { .. })
    }

    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            AuditStatus::Found { path, .. } => Some(path),
            AuditStatus::Missing => None,
        }
    }
}

impl fmt::Display for AuditStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditStatus::Found { path, link } => {
                let link_str = link.display_short();
                if link_str.is_empty() {
                    write!(f, "✔ Vorhanden  {}", path.display())
                } else {
                    write!(f, "{} {}  {}", link.icon(), path.display(), link_str)
                }
            }
            AuditStatus::Missing => write!(f, "✘ Fehlt      ---"),
        }
    }
}

// -------------------------------------------------------

/// Metadaten zu einem Tool (statisch, zur Kompilierzeit bekannt)
#[derive(Debug, Clone)]
pub struct ToolMeta {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

// -------------------------------------------------------

/// Das Ergebnis einer einzelnen Werkzeug-Prüfung (Laufzeit)
#[derive(Debug)]
pub struct CommandAudit {
    pub meta: ToolMeta,
    pub status: AuditStatus,
}

impl CommandAudit {
    /// Prüft, ob das Tool im PATH vorhanden ist
    pub fn check(meta: ToolMeta) -> Self {
        let status = match which(meta.name) {
            Ok(path) => {
                let link = LinkKind::detect(&path);
                AuditStatus::Found { path, link }
            }
            Err(_) => AuditStatus::Missing,
        };
        Self { meta, status }
    }

    /// Kurzform für ad-hoc Prüfungen ohne vollständige Metadaten
    pub fn check_simple(name: &'static str) -> Self {
        Self::check(ToolMeta {
            name,
            category: "Unbekannt",
            description: "—",
        })
    }

    pub fn is_available(&self) -> bool {
        self.status.is_found()
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.status.path()
    }
}

// -------------------------------------------------------
// Ausgabe-Formatierung
// -------------------------------------------------------

/// Gibt eine Liste von Audits gruppiert nach Kategorie aus
pub fn print_audit_report(audits: &[CommandAudit]) {
    // Kategorien in Reihenfolge des ersten Auftretens sammeln
    let mut categories: Vec<&str> = Vec::new();
    for a in audits {
        if !categories.contains(&a.meta.category) {
            categories.push(a.meta.category);
        }
    }

    let col_name = 16usize;
    let col_path = 42usize;
    let col_desc = 30usize;
    let total = col_name + col_path + col_desc + 6;

    for cat in categories {
        println!("\n  [{cat}]");
        println!("{:=<total$}", "");
        println!("{:<col_name$}  {:<col_path$}  {:<col_desc$}", "Tool", "Status / Pfad", "Beschreibung");
        println!("{:-<total$}", "");

        for audit in audits.iter().filter(|a| a.meta.category == cat) {
            let (status_icon, path_str) = match &audit.status {
                AuditStatus::Found { path, link } => {
                    let link_display = link.display_short();
                    let full_path = if link_display.is_empty() {
                        path.display().to_string()
                    } else {
                        format!("{} {}", path.display(), link_display)
                    };
                    (link.icon(), full_path)
                }
                AuditStatus::Missing => ("✘", "---".to_string()),
            };

            println!(
                "{:<col_name$}  {} {:<col_path$}  {:<col_desc$}",
                audit.meta.name,
                status_icon,
                path_str,
                audit.meta.description,
            );
        }
    }
    println!();
}

// -------------------------------------------------------
// Tool-Registry (erweiterbar, kein Duplikat-Problem)
// -------------------------------------------------------

pub fn default_package_managers() -> Vec<ToolMeta> {
    vec![
        // === System Package Manager ===
        ToolMeta { name: "dnf", category: "System", description: "Fedora/RHEL modern" },
        ToolMeta { name: "dnf5", category: "System", description: "Fedora/RHEL next-gen" },
        ToolMeta { name: "yum", category: "System", description: "RHEL/CentOS legacy" },
        ToolMeta { name: "rpm", category: "System", description: "RPM low-level" },
        ToolMeta { name: "rpm-ostree", category: "System", description: "Bazzite/Silverblue" },
        ToolMeta { name: "apt", category: "System", description: "Debian/Ubuntu" },
        ToolMeta { name: "apt-get", category: "System", description: "Debian/Ubuntu classic" },
        ToolMeta { name: "aptitude", category: "System", description: "Debian advanced" },
        ToolMeta { name: "dpkg", category: "System", description: "Debian low-level" },
        ToolMeta { name: "pacman", category: "System", description: "Arch Linux" },
        ToolMeta { name: "yay", category: "System", description: "Arch AUR helper" },
        ToolMeta { name: "paru", category: "System", description: "Arch AUR helper" },
        ToolMeta { name: "pamac", category: "System", description: "Manjaro" },
        ToolMeta { name: "zypper", category: "System", description: "openSUSE" },
        ToolMeta { name: "emerge", category: "System", description: "Gentoo" },
        ToolMeta { name: "xbps-install", category: "System", description: "Void Linux" },
        ToolMeta { name: "apk", category: "System", description: "Alpine Linux" },
        ToolMeta { name: "eopkg", category: "System", description: "Solus" },
        ToolMeta { name: "swupd", category: "System", description: "Clear Linux" },
        ToolMeta { name: "guix", category: "System", description: "GNU Guix" },
        ToolMeta { name: "nix-env", category: "System", description: "NixOS/Nix" },
        ToolMeta { name: "nix", category: "System", description: "Nix CLI (flakes)" },
        ToolMeta { name: "pacstall", category: "System", description: "Ubuntu AUR-like" },
        ToolMeta { name: "slackpkg", category: "System", description: "Slackware" },
        ToolMeta { name: "slapt-get", category: "System", description: "Slackware APT-like" },
        ToolMeta { name: "tazpkg", category: "System", description: "SliTaz" },
        ToolMeta { name: "opkg", category: "System", description: "OpenWRT/Embedded" },
        ToolMeta { name: "pkg", category: "System", description: "FreeBSD / Termux" },
        ToolMeta { name: "pkgin", category: "System", description: "NetBSD pkgsrc" },
        ToolMeta { name: "pkg_add", category: "System", description: "OpenBSD" },
        ToolMeta { name: "port", category: "System", description: "MacPorts" },
        ToolMeta { name: "ostree", category: "System", description: "OS image version control (Git-like)" },
        ToolMeta { name: "microdnf", category: "System", description: "Minimal C-based DNF" },
        ToolMeta { name: "pkcon", category: "System", description: "PackageKit (GUI Backend)" },

        // === Universal / Container Apps ===
        ToolMeta { name: "flatpak", category: "Universal", description: "Flatpak apps" },
        ToolMeta { name: "snap", category: "Universal", description: "Snap apps" },
        ToolMeta { name: "brew", category: "Universal", description: "Homebrew" },
        ToolMeta { name: "appimage", category: "Universal", description: "AppImage" },
        ToolMeta { name: "distrobox", category: "Universal", description: "Container environments" },
        ToolMeta { name: "buildah", category: "Universal", description: "OCI image builder" },
        ToolMeta { name: "bootc", category: "Universal", description: "Bootable containers" },
        ToolMeta { name: "appstreamcli", category: "Universal", description: "AppStream metadata" },
        ToolMeta { name: "flatpak-spawn", category: "Universal", description: "Escape Flatpak sandbox" },

        // === Sprach-Ökosysteme ===
        ToolMeta { name: "cargo", category: "Language", description: "Rust" },
        ToolMeta { name: "rustup", category: "Language", description: "Rust toolchain mgr" },
        ToolMeta { name: "go", category: "Language", description: "Go" },
        ToolMeta { name: "npm", category: "Language", description: "Node.js" },
        ToolMeta { name: "pnpm", category: "Language", description: "Node.js fast" },
        ToolMeta { name: "yarn", category: "Language", description: "Node.js Yarn" },
        ToolMeta { name: "bun", category: "Language", description: "Bun JS runtime" },
        ToolMeta { name: "deno", category: "Language", description: "Deno runtime" },
        ToolMeta { name: "pip", category: "Language", description: "Python 2" },
        ToolMeta { name: "pip3", category: "Language", description: "Python 3" },
        ToolMeta { name: "pipx", category: "Language", description: "Python isolated apps" },
        ToolMeta { name: "pipenv", category: "Language", description: "Python env mgr" },
        ToolMeta { name: "uv", category: "Language", description: "Python fast (Astral)" },
        ToolMeta { name: "poetry", category: "Language", description: "Python dep mgr" },
        ToolMeta { name: "conda", category: "Language", description: "Anaconda/Miniconda" },
        ToolMeta { name: "mamba", category: "Language", description: "Conda fast" },
        ToolMeta { name: "gem", category: "Language", description: "Ruby gems" },
        ToolMeta { name: "bundler", category: "Language", description: "Ruby bundler" },
        ToolMeta { name: "composer", category: "Language", description: "PHP Composer" },
        ToolMeta { name: "dotnet", category: "Language", description: ".NET / NuGet" },
        ToolMeta { name: "java", category: "Language", description: "Java JVM" },
        ToolMeta { name: "mvn", category: "Language", description: "Maven (Java)" },
        ToolMeta { name: "gradle", category: "Language", description: "Gradle (Java/Kotlin)" },
        ToolMeta { name: "swift", category: "Language", description: "Swift / SPM" },
        ToolMeta { name: "dart", category: "Language", description: "Dart/Flutter" },
        ToolMeta { name: "pub", category: "Language", description: "Dart pub" },
        ToolMeta { name: "luarocks", category: "Language", description: "Lua rocks" },
        ToolMeta { name: "mix", category: "Language", description: "Elixir Mix" },
        ToolMeta { name: "hex", category: "Language", description: "Hex (Elixir)" },
        ToolMeta { name: "opam", category: "Language", description: "OCaml" },
        ToolMeta { name: "cabal", category: "Language", description: "Haskell Cabal" },
        ToolMeta { name: "stack", category: "Language", description: "Haskell Stack" },
        ToolMeta { name: "ghcup", category: "Language", description: "Haskell GHCup" },
        ToolMeta { name: "julia", category: "Language", description: "Julia Pkg" },
        ToolMeta { name: "R", category: "Language", description: "R CRAN" },
        ToolMeta { name: "vcpkg", category: "Language", description: "C/C++ vcpkg" },
        ToolMeta { name: "conan", category: "Language", description: "C/C++ Conan" },
        ToolMeta { name: "nimble", category: "Language", description: "Nim" },
        ToolMeta { name: "zig", category: "Language", description: "Zig build system" },

        // === Task Runner / Build ===
        ToolMeta { name: "just", category: "Build", description: "Just task runner" },
        ToolMeta { name: "ujust", category: "Build", description: "uBlue just wrapper" },
        ToolMeta { name: "make", category: "Build", description: "GNU Make" },
        ToolMeta { name: "cmake", category: "Build", description: "CMake" },
        ToolMeta { name: "meson", category: "Build", description: "Meson build" },
        ToolMeta { name: "ninja", category: "Build", description: "Ninja build" },
        ToolMeta { name: "bazel", category: "Build", description: "Bazel (Google)" },
        ToolMeta { name: "buck2", category: "Build", description: "Buck2 (Meta)" },
    ]
}

// -------------------------------------------------------
// Einstiegspunkt
// -------------------------------------------------------

fn main() {
    // 1. Registry holen
    let tools = default_package_managers();

    // 2. Alle prüfen (dedupliziert durch Registry-Design)
    let audits: Vec<CommandAudit> = tools
        .into_iter()
        .map(CommandAudit::check)
        .collect();

    // 3. Bericht ausgeben
    print_audit_report(&audits);

    // 4. Weiterverarbeitung bleibt einfach:
    let available: Vec<&CommandAudit> = audits.iter()
        .filter(|a| a.is_available())
        .collect();

    println!("Zusammenfassung: {}/{} Tools gefunden.", available.len(), audits.len());
}