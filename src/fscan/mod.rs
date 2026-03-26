pub mod core;   // Shared types: FileKind, ExecutableKind, classify_path, ...
pub mod magic;  // Magic-Number-Trie — einzige Erkennungsquelle für alle Module
pub mod probe;  // UseCase 1: Tool-Probing by name (which + classify)
pub mod scan;   // UseCase 2: Directory scanning by path