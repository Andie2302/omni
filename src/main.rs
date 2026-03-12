mod system_info;

use std::fs;

fn main() {
    let system_info = system_info::SystemInfo::new();
    println!("Der Name von Prozess ID 1 ist: '{:?}'", system_info.init);

    match system_info.init {
        system_info::InitType::Systemd => println!("Logik: Ich erstelle .mount Dateien."),
        system_info::InitType::OpenRc => println!("Logik: Ich nutze die /etc/fstab."),
        system_info::InitType::Unknown(init_name) => println!("Logik: Unbekannter Init-Prozess: {}", init_name),
    }
}