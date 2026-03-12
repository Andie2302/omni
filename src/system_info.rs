use std::fs;

const INIT_COMM_PATH: &str = "/proc/1/comm";

pub struct SystemInfo {
    pub init: InitType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitType {
    Systemd,
    OpenRc,
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
            "openrc" => InitType::OpenRc,
            _ => InitType::Unknown(init_name),
        }
    }
}