//! Shared types — used by both `probe` (UseCase 1) and `scan` (UseCase 2).
//!
//! ## Erkennungsstrategie
//! Wie ein IR-Empfänger: wir lesen maximal 18 Bytes vom Dateianfang
//! und matchen gegen bekannte Magic-Number-Muster.
//! Nie mehr Bytes als nötig, nie den ganzen Dateiinhalt.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

// -------------------------------------------------------
// Bits
// -------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Bits {
    B32,
    B64,
    Unknown,
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::B32 => write!(f, "32-bit"),
            Bits::B64 => write!(f, "64-bit"),
            Bits::Unknown => write!(f, "?-bit"),
        }
    }
}

// -------------------------------------------------------
// ElfType
// -------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ElfType {
    Executable,    // ET_EXEC = 2
    SharedObject,  // ET_DYN  = 3  (auch PIE-Binaries!)
    Core,          // ET_CORE = 4
    Other(u16),
}

impl fmt::Display for ElfType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElfType::Executable => write!(f, "exec"),
            ElfType::SharedObject => write!(f, "dyn/PIE"),
            ElfType::Core => write!(f, "core"),
            ElfType::Other(n) => write!(f, "e_type={n}"),
        }
    }
}

// -------------------------------------------------------
// BinaryFormat
// -------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryFormat {
    Elf { bits: Bits, elf_type: ElfType },
    MachO { bits: Bits },
    Pe,
    Wasm,
    Unknown,
}

impl fmt::Display for BinaryFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryFormat::Elf { bits, elf_type } => write!(f, "ELF {bits} {elf_type}"),
            BinaryFormat::MachO { bits } => write!(f, "Mach-O {bits}"),
            BinaryFormat::Pe => write!(f, "PE (Windows)"),
            BinaryFormat::Wasm => write!(f, "WebAssembly"),
            BinaryFormat::Unknown => write!(f, "binary/unknown"),
        }
    }
}

// -------------------------------------------------------
// Privileges — Dateisystem-Rechte, unabhängig vom Format
// -------------------------------------------------------

/// SUID/SGID sind Dateisystem-Eigenschaften, kein ELF-Konzept.
/// Sie werden separat von BinaryFormat erfasst und dann kombiniert.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Privileges {
    pub suid: bool,   // 0o4000 — läuft als Datei-Eigentümer (z.B. root)
    pub sgid: bool,   // 0o2000 — läuft mit Gruppen-ID der Datei
}

impl Privileges {
    pub fn is_elevated(&self) -> bool {
        self.suid || self.sgid
    }
}

impl fmt::Display for Privileges {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.suid, self.sgid) {
            (true, true) => write!(f, " ⚠ SUID+SGID"),
            (true, false) => write!(f, " ⚠ SUID"),
            (false, true) => write!(f, " ⚠ SGID"),
            (false, false) => Ok(()),
        }
    }
}

// -------------------------------------------------------
// ExecutableKind
// -------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutableKind {
    NativeBinary { format: BinaryFormat, privileges: Privileges },
    Script { interpreter: String },
    Unknown,
}

impl fmt::Display for ExecutableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutableKind::NativeBinary { format, privileges } =>
                write!(f, "{format}{privileges}"),
            ExecutableKind::Script { interpreter } =>
                write!(f, "script ({interpreter})"),
            ExecutableKind::Unknown =>
                write!(f, "executable/unknown"),
        }
    }
}

impl ExecutableKind {
    pub fn is_elf(&self) -> bool {
        matches!(self, ExecutableKind::NativeBinary { format: BinaryFormat::Elf { .. }, .. })
    }

    pub fn elf_info(&self) -> Option<(&Bits, &ElfType)> {
        match self {
            ExecutableKind::NativeBinary {
                format: BinaryFormat::Elf { bits, elf_type }, ..
            } => Some((bits, elf_type)),
            _ => None,
        }
    }

    pub fn is_elevated(&self) -> bool {
        matches!(self, ExecutableKind::NativeBinary { privileges, .. } if privileges.is_elevated())
    }
}

// -------------------------------------------------------
// FileKind
// -------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    Executable(ExecutableKind),
    NonExecutable,
    Unknown(String),
}

impl FileKind {
    pub fn is_executable(&self) -> bool {
        matches!(self, FileKind::Executable(_))
    }

    pub fn executable_kind(&self) -> Option<&ExecutableKind> {
        match self {
            FileKind::Executable(k) => Some(k),
            _ => None,
        }
    }
}

impl fmt::Display for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileKind::Executable(k) => write!(f, "{k}"),
            FileKind::NonExecutable => write!(f, "non-executable"),
            FileKind::Unknown(r) => write!(f, "unknown ({r})"),
        }
    }
}

// -------------------------------------------------------
// LinkKind
// -------------------------------------------------------

#[derive(Debug, Clone)]
pub enum LinkKind {
    Direct,
    Symlink { target: PathBuf },
    SymlinkChain { chain: Vec<PathBuf> },
}

impl LinkKind {
    pub fn detect(path: &Path) -> Self {
        let chain = Self::resolve_chain(path);
        match chain.len() {
            0 => LinkKind::Direct,
            1 => LinkKind::Symlink { target: chain.into_iter().next().unwrap() },
            _ => LinkKind::SymlinkChain { chain },
        }
    }

    pub fn display_short(&self) -> String {
        match self {
            LinkKind::Direct => String::new(),
            LinkKind::Symlink { target } => format!("→ {}", target.display()),
            LinkKind::SymlinkChain { chain } => {
                let last = chain.last().unwrap();
                format!("→→ {} ({} hops)", last.display(), chain.len())
            }
        }
    }

    fn resolve_chain(path: &Path) -> Vec<PathBuf> {
        let mut chain = Vec::new();
        let mut current = path.to_path_buf();
        for _ in 0..10 {
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
// classify_path — gemeinsam für probe + scan
// -------------------------------------------------------

pub fn classify_path(path: &Path) -> FileKind {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return FileKind::Unknown(e.to_string()),
    };
    if !meta.is_file() {
        return FileKind::NonExecutable;
    }
    if is_executable_meta(&meta) {
        FileKind::Executable(detect_executable_kind(path))
    } else {
        FileKind::NonExecutable
    }
}

// -------------------------------------------------------
// Interne Hilfsfunktionen
// -------------------------------------------------------

#[cfg(unix)]
fn is_executable_meta(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_meta(_meta: &fs::Metadata) -> bool {
    false
}

fn detect_executable_kind(path: &Path) -> ExecutableKind {
    // Schritt 1: Privileges aus Dateisystem-Metadaten lesen (unabhängig vom Inhalt)
    let privileges = read_privileges(path);

    // Schritt 2: Format aus den ersten 18 Bytes erkennen
    let mut header = [0u8; 18];
    let n = match read_header(path, &mut header) {
        Ok(n) if n >= 2 => n,
        _ => return ExecutableKind::Unknown,
    };

    // Schritt 3: Magic-Match — Scripts bekommen keine Privileges
    let kind = match_magic(&header[..n]);

    // Schritt 4: Privileges nur bei NativeBinary einsetzen
    // Scripts (Shebang) laufen unter dem Interpreter, nicht SUID
    match kind {
        ExecutableKind::NativeBinary { format, .. } =>
            ExecutableKind::NativeBinary { format, privileges },
        other => other,
    }
}

/// Öffnet die Datei, liest maximal buf.len() Bytes, schließt sie sofort.
fn read_header(path: &Path, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut f = fs::File::open(path)?;
    Ok(f.read(buf)?)
}

/// Liest SUID/SGID aus den Dateisystem-Metadaten — kein Dateiinhalt nötig.
#[cfg(unix)]
fn read_privileges(path: &Path) -> Privileges {
    use std::os::unix::fs::PermissionsExt;
    match fs::metadata(path) {
        Ok(meta) => {
            let mode = meta.permissions().mode();
            Privileges {
                suid: mode & 0o4000 != 0,
                sgid: mode & 0o2000 != 0,
            }
        }
        Err(_) => Privileges::default(),
    }
}

#[cfg(not(unix))]
fn read_privileges(_path: &Path) -> Privileges {
    Privileges::default()
}

/// Magic-Number-Matching — spezifischste Muster zuerst.
fn match_magic(h: &[u8]) -> ExecutableKind {

    // Shebang muss vor allen Binary-Checks kommen
    if h.starts_with(b"#!") {
        return parse_shebang(h);
    }

    // ELF: 7F 45 4C 46
    if h.starts_with(b"\x7FELF") {
        return parse_elf(h);
    }

    // Mach-O: alle 5 Varianten
    let macho_bits = match h.get(..4) {
        Some(b"\xCE\xFA\xED\xFE") => Some(Bits::B32),     // 32-bit LE
        Some(b"\xCF\xFA\xED\xFE") => Some(Bits::B64),     // 64-bit LE
        Some(b"\xFE\xED\xFA\xCE") => Some(Bits::B32),     // 32-bit BE
        Some(b"\xFE\xED\xFA\xCF") => Some(Bits::B64),     // 64-bit BE
        Some(b"\xCA\xFE\xBA\xBE") => Some(Bits::Unknown), // Fat/Universal
        _ => None,
    };
    if let Some(bits) = macho_bits {
        return ExecutableKind::NativeBinary {
            format: BinaryFormat::MachO { bits },
            privileges: Privileges::default(),
        };
    }

    // PE / Windows: MZ (4D 5A)
    if h.starts_with(b"MZ") {
        return ExecutableKind::NativeBinary {
            format: BinaryFormat::Pe,
            privileges: Privileges::default(),
        };
    }

    // WebAssembly: \0asm (00 61 73 6D)
    if h.starts_with(b"\x00asm") {
        return ExecutableKind::NativeBinary {
            format: BinaryFormat::Wasm,
            privileges: Privileges::default(),
        };
    }

    // x-Bit gesetzt, aber kein bekanntes Magic
    ExecutableKind::NativeBinary {
        format: BinaryFormat::Unknown,
        privileges: Privileges::default(),
    }
}

fn parse_elf(h: &[u8]) -> ExecutableKind {
    // Byte 4: EI_CLASS
    let bits = match h.get(4) {
        Some(1) => Bits::B32,
        Some(2) => Bits::B64,
        _ => Bits::Unknown,
    };

    // Byte 5: EI_DATA — 1=LE, 2=BE
    let little_endian = h.get(5).copied().unwrap_or(1) == 1;

    // Byte 16-17: e_type
    let elf_type = if h.len() >= 18 {
        let raw = if little_endian {
            u16::from_le_bytes([h[16], h[17]])
        } else {
            u16::from_be_bytes([h[16], h[17]])
        };
        match raw {
            2 => ElfType::Executable,
            3 => ElfType::SharedObject,
            4 => ElfType::Core,
            n => ElfType::Other(n),
        }
    } else {
        ElfType::Other(0)
    };

    ExecutableKind::NativeBinary {
        format: BinaryFormat::Elf { bits, elf_type },
        privileges: Privileges::default(), // wird später von detect_executable_kind überschrieben
    }
}

fn parse_shebang(h: &[u8]) -> ExecutableKind {
    let text = match std::str::from_utf8(h) {
        Ok(s) => s,
        Err(_) => return ExecutableKind::Unknown,
    };
    let line = text.lines().next().unwrap_or("");
    let after = line.trim_start_matches("#!").trim();
    let raw = after.split_whitespace().next().unwrap_or("unknown");
    let interpreter = Path::new(raw)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(raw)
        .to_string();
    ExecutableKind::Script { interpreter }
}

// -------------------------------------------------------
// Tests
// -------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn elf_header(class: u8, data: u8, e_type: u16) -> [u8; 18] {
        let mut h = [0u8; 18];
        h[0..4].copy_from_slice(b"\x7FELF");
        h[4] = class;
        h[5] = data;
        let bytes = if data == 1 {
            e_type.to_le_bytes()
        } else {
            e_type.to_be_bytes()
        };
        h[16] = bytes[0];
        h[17] = bytes[1];
        h
    }

    #[test]
    fn elf64_exec_le() {
        let h = elf_header(2, 1, 2);
        assert!(matches!(
            match_magic(&h),
            ExecutableKind::NativeBinary {
                format: BinaryFormat::Elf { bits: Bits::B64, elf_type: ElfType::Executable },
                ..
            }
        ));
    }

    #[test]
    fn elf32_dyn_le() {
        let h = elf_header(1, 1, 3);
        assert!(matches!(
            match_magic(&h),
            ExecutableKind::NativeBinary {
                format: BinaryFormat::Elf { bits: Bits::B32, elf_type: ElfType::SharedObject },
                ..
            }
        ));
    }

    #[test]
    fn elf64_be() {
        let h = elf_header(2, 2, 2);
        assert!(matches!(
            match_magic(&h),
            ExecutableKind::NativeBinary {
                format: BinaryFormat::Elf { bits: Bits::B64, elf_type: ElfType::Executable },
                ..
            }
        ));
    }

    #[test]
    fn shebang_env_python() {
        assert_eq!(
            match_magic(b"#!/usr/bin/env python3\n"),
            ExecutableKind::Script { interpreter: "env".into() }
        );
    }

    #[test]
    fn shebang_direct_bash() {
        assert_eq!(
            match_magic(b"#!/bin/bash\n"),
            ExecutableKind::Script { interpreter: "bash".into() }
        );
    }

    #[test]
    fn pe_mz() {
        assert!(matches!(
            match_magic(b"MZ\x90\x00rest"),
            ExecutableKind::NativeBinary { format: BinaryFormat::Pe, .. }
        ));
    }

    #[test]
    fn wasm() {
        assert!(matches!(
            match_magic(b"\x00asm\x01\x00\x00\x00"),
            ExecutableKind::NativeBinary { format: BinaryFormat::Wasm, .. }
        ));
    }

    #[test]
    fn macho_64le() {
        assert!(matches!(
            match_magic(b"\xCF\xFA\xED\xFE\x07\x00"),
            ExecutableKind::NativeBinary {
                format: BinaryFormat::MachO { bits: Bits::B64 },
                ..
            }
        ));
    }

    #[test]
    fn display_elf64_exec() {
        let k = ExecutableKind::NativeBinary {
            format: BinaryFormat::Elf { bits: Bits::B64, elf_type: ElfType::Executable },
            privileges: Privileges::default(),
        };
        assert_eq!(k.to_string(), "ELF 64-bit exec");
    }

    #[test]
    fn display_script() {
        let k = ExecutableKind::Script { interpreter: "python3".into() };
        assert_eq!(k.to_string(), "script (python3)");
    }

    #[test]
    fn display_suid() {
        let k = ExecutableKind::NativeBinary {
            format: BinaryFormat::Elf { bits: Bits::B64, elf_type: ElfType::Executable },
            privileges: Privileges { suid: true, sgid: false },
        };
        assert_eq!(k.to_string(), "ELF 64-bit exec ⚠ SUID");
    }

    #[test]
    fn display_suid_sgid() {
        let k = ExecutableKind::NativeBinary {
            format: BinaryFormat::Elf { bits: Bits::B64, elf_type: ElfType::SharedObject },
            privileges: Privileges { suid: true, sgid: true },
        };
        assert_eq!(k.to_string(), "ELF 64-bit dyn/PIE ⚠ SUID+SGID");
    }

    #[test]
    fn elf_info_helper() {
        let k = ExecutableKind::NativeBinary {
            format: BinaryFormat::Elf { bits: Bits::B64, elf_type: ElfType::Executable },
            privileges: Privileges::default(),
        };
        let (bits, elf_type) = k.elf_info().unwrap();
        assert_eq!(bits, &Bits::B64);
        assert_eq!(elf_type, &ElfType::Executable);
    }
}