
/*


pub fn default_package_managers<ToolMeta>() -> Vec<ToolMeta> {
    vec![

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

        ToolMeta { name: "flatpak", category: "Universal", description: "Flatpak apps" },
        ToolMeta { name: "snap", category: "Universal", description: "Snap apps" },
        ToolMeta { name: "brew", category: "Universal", description: "Homebrew" },
        ToolMeta { name: "appimage", category: "Universal", description: "AppImage" },
        ToolMeta { name: "distrobox", category: "Universal", description: "Container environments" },
        ToolMeta { name: "buildah", category: "Universal", description: "OCI image builder" },
        ToolMeta { name: "bootc", category: "Universal", description: "Bootable containers" },
        ToolMeta { name: "appstreamcli", category: "Universal", description: "AppStream metadata" },
        ToolMeta { name: "flatpak-spawn", category: "Universal", description: "Escape Flatpak sandbox" },

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

*/
use omni_tools::command::executor::PrintHandler;
use omni_tools::{ExecutorConfig, OmniCommand, OmniCommandArg, OmniExecutor};
use std::sync::Arc;

fn main() {
    // --- SCHRITT 1: Das Kommando zusammenbauen ---
    // Wir erstellen 'flatpak list --app'
    let cmd = OmniCommand::new("flatpak")
        .with_arg(OmniCommandArg::new("list"))
        .with_arg(OmniCommandArg::new("--app"));

    println!("Starte Befehl: {}", cmd);

    // --- SCHRITT 2: Den Executor vorbereiten ---
    // Der PrintHandler gibt alles sofort im Terminal aus.
    let handler = Arc::new(PrintHandler);
    let config = ExecutorConfig::default(); // Standard: Live-Ausgabe, kein Dry-Run

    let executor = OmniExecutor::new(config, handler);

    // --- SCHRITT 3: Ausführen ---
    let result = executor.execute(&cmd);

    // Ergebnis prüfen
    if result.is_success() {
        println!("\n✅ Befehl erfolgreich ausgeführt.");
    } else {
        println!("\n❌ Fehler: {}", result.status_message());
    }
}