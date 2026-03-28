#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magic {
    // --- Executables (werden von core.rs genutzt) ---
    Elf,
    MachO32Le,
    MachO64Le,
    MachO32Be,
    MachO64Be,
    MachoFat,    // Universal / Fat Binary
    Pe,          // Windows MZ
    Wasm,
    Shebang,     // #! — Interpreter wird separat geparst

    // --- Bilder ---
    Jpeg,
    Png,
    Gif87a,
    Gif89a,
    Bmp,
    TiffLe,      // little-endian
    TiffBe,      // big-endian
    WebP,        // "RIFF????WEBP" — Offset-Match

    // --- Dokumente ---
    Pdf,

    // --- Archive ---
    Zip,         // auch DOCX, JAR, APK, ...
    Rar,
    SevenZip,
    Gz,
    Bz2,
    Xz,

    // --- Audio / Video ---
    Flac,
    Ogg,
    Mp3Id3,
    Mp4,         // ftyp-Box an Offset 4 — Offset-Match
    Mkv,         // EBML-Header
}

#[derive(Default)]
struct TrieNode {
    children: std::collections::HashMap<u8, TrieNode>,
    result: Option<Magic>,
}

impl TrieNode {
    fn insert(&mut self, bytes: &[u8], magic: Magic) {
        if bytes.is_empty() {
            self.result = Some(magic);
            return;
        }
        self.children
            .entry(bytes[0])
            .or_default()
            .insert(&bytes[1..], magic);
    }

    fn search(&self, data: &[u8]) -> Option<Magic> {
        if let Some(m) = self.result {
            return Some(m);
        }
        let (&first, rest) = data.split_first()?;
        self.children.get(&first)?.search(rest)
    }
}

static SIGNATURES: &[(&[u8], Magic)] = &[
    // Executables
    (b"#!",                          Magic::Shebang),
    (b"\x7FELF",                     Magic::Elf),
    (b"\xCE\xFA\xED\xFE",           Magic::MachO32Le),
    (b"\xCF\xFA\xED\xFE",           Magic::MachO64Le),
    (b"\xFE\xED\xFA\xCE",           Magic::MachO32Be),
    (b"\xFE\xED\xFA\xCF",           Magic::MachO64Be),
    (b"\xCA\xFE\xBA\xBE",           Magic::MachoFat),
    (b"MZ",                          Magic::Pe),
    (b"\x00asm",                     Magic::Wasm),
    // Bilder
    (b"\xFF\xD8\xFF",                Magic::Jpeg),
    (b"\x89PNG\r\n\x1a\n",          Magic::Png),
    (b"GIF87a",                      Magic::Gif87a),
    (b"GIF89a",                      Magic::Gif89a),
    (b"BM",                          Magic::Bmp),
    (b"\x49\x49\x2A\x00",           Magic::TiffLe),
    (b"\x4D\x4D\x00\x2A",           Magic::TiffBe),
    // Dokumente
    (b"%PDF",                        Magic::Pdf),
    // Archive
    (b"PK\x03\x04",                 Magic::Zip),
    (b"Rar!\x1a\x07",               Magic::Rar),
    (b"7z\xBC\xAF\x27\x1C",        Magic::SevenZip),
    (b"\x1F\x8B",                    Magic::Gz),
    (b"BZh",                         Magic::Bz2),
    (b"\xFD7zXZ\x00",               Magic::Xz),
    // Audio / Video
    (b"fLaC",                        Magic::Flac),
    (b"OggS",                        Magic::Ogg),
    (b"ID3",                         Magic::Mp3Id3),
    (b"\x1A\x45\xDF\xA3",           Magic::Mkv),
];

use std::sync::OnceLock;

fn trie() -> &'static TrieNode {
    static TRIE: OnceLock<TrieNode> = OnceLock::new();
    TRIE.get_or_init(|| {
        let mut root = TrieNode::default();
        for &(bytes, magic) in SIGNATURES {
            root.insert(bytes, magic);
        }
        root
    })
}

pub fn detect(data: &[u8]) -> Option<Magic> {
    if let Some(m) = trie().search(data) {
        return Some(m);
    }
    match_offset_magic(data)
}

fn match_offset_magic(data: &[u8]) -> Option<Magic> {
    if data.len() >= 12
        && data.starts_with(b"RIFF")
        && &data[8..12] == b"WEBP"
    {
        return Some(Magic::WebP);
    }
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return Some(Magic::Mp4);
    }
    None
}

pub fn detect_file(path: &std::path::Path) -> std::io::Result<Option<Magic>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = [0u8; 16];
    let n = f.read(&mut buf)?;
    Ok(detect(&buf[..n]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png() {
        assert_eq!(detect(b"\x89PNG\r\n\x1a\nrest"), Some(Magic::Png));
    }

    #[test]
    fn jpeg() {
        assert_eq!(detect(b"\xFF\xD8\xFF\xE0rest"), Some(Magic::Jpeg));
    }

    #[test]
    fn elf() {
        assert_eq!(detect(b"\x7FELFrest"), Some(Magic::Elf));
    }

    #[test]
    fn shebang() {
        assert_eq!(detect(b"#!/bin/bash\n"), Some(Magic::Shebang));
    }

    #[test]
    fn pe() {
        assert_eq!(detect(b"MZ\x90\x00rest"), Some(Magic::Pe));
    }

    #[test]
    fn wasm() {
        assert_eq!(detect(b"\x00asm\x01\x00\x00\x00"), Some(Magic::Wasm));
    }

    #[test]
    fn webp_offset() {
        let data = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(detect(data), Some(Magic::WebP));
    }

    #[test]
    fn mp4_offset() {
        assert_eq!(detect(b"\x00\x00\x00\x20ftypisom"), Some(Magic::Mp4));
    }

    #[test]
    fn gif_versions() {
        assert_eq!(detect(b"GIF87aXXX"), Some(Magic::Gif87a));
        assert_eq!(detect(b"GIF89aXXX"), Some(Magic::Gif89a));
    }

    #[test]
    fn unknown() {
        assert_eq!(detect(b"\x00\x00\x00\x00"), None);
    }
}
