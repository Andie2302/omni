use crate::linux_command::LinuxCommand;

mod linux_command;

fn main() -> std::io::Result<()> {
    let cmd = LinuxCommand::new("rpm")
        .flag("-", "q")
        .flag("-", "a");

    println!("Starte Suche mit: {}", cmd);

    let all_packages = cmd.output_string()?;
    let search_term = "nvidia";
    let found_packages: Vec<&str> = all_packages
        .lines()
        .filter(|line| line.contains(search_term))
        .collect();
    if found_packages.is_empty() {
        println!("Keine Pakete mit '{}' gefunden.", search_term);
    } else {
        println!("Gefundene Pakete ({}):", found_packages.len());
        for pkg in found_packages {
            println!(" - {}", pkg);
        }
    }

    Ok(())
}