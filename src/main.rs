use test::list_block_devices;

mod system_info;
mod test;

fn main() {
    let system_info = system_info::SystemInfo::new();
    println!("Der Name von Prozess ID 1 ist: '{:?}'", system_info.init);

    match system_info.init {
        system_info::InitType::Systemd => println!("Logik: Ich erstelle .mount Dateien."),
        system_info::InitType::Unknown(init_name) => println!("Logik: Unbekannter Init-Prozess: {}", init_name),
    }

    let list = list_block_devices();

    for device in list {
        println!("{:?}", device);
        println!("{:?}", device.mountpoint);
        println!("{:?}", device.label);
        println!("{:?}", device.name);
        println!("{:?}", device.uuid);
        println!("{:?}", device.path);
        println!();
    }

}