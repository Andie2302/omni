use std::fs;
use std::process::Command;

const INIT_COMM_PATH: &str = "/proc/1/comm";

pub struct SystemInfo {
    pub init: InitType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitType {
    Systemd,
    Unknown(String),
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            init: Self::read_init_type(),
        }
    }

    fn read_init_type() -> InitType {
        let init_name = fs::read_to_string(INIT_COMM_PATH)
            .map(|content| content.trim().to_owned())
            .unwrap_or_else(|_| "unknown".to_owned());

        Self::parse_init_type(init_name)
    }

    fn parse_init_type(init_name: String) -> InitType {
        match init_name.as_str() {
            "systemd" => InitType::Systemd,
            _ => InitType::Unknown(init_name),
        }
    }
}

pub fn list_dev_uuids() {
    // Wir rufen 'lsblk' auf mit:
    // -n: keine Überschriften
    // -o: Spalten auswählen (NAME, MOUNTPOINT, UUID)
    let output = Command::new("lsblk")
        .args(&["-no", "NAME,MOUNTPOINT,UUID"])
        .output()
        .expect("Fehler: lsblk konnte nicht gestartet werden");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        println!("Gefundene Partitionen:");
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();

            // Eine Zeile mit UUID hat meistens 3 Teile (Name, [Mountpoint], UUID)
            // Da der Mountpoint leer sein kann, prüfen wir die Länge
            if parts.len() >= 2 {
                let name = parts[0];
                let uuid = parts.last().unwrap_or(&"Keine UUID");
                println!("Gerät: /dev/{} -> UUID: {}", name, uuid);
            }
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Fehler beim Ausführen von lsblk: {}", stderr);
    }
}