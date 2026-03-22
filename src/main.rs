use crate::linux_command::{LinuxCommand, Argument};
use std::error::Error;

mod linux_command;
mod error;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    println!("--- LinuxCommand Test Suite ---\n");

    // 1. Einfacher Befehl: Directory Listing
    // Entspricht: ls -l -a
    let ls_cmd = LinuxCommand::new("ls")
        .flag("-", "l")
        .flag("-", "a");

    println!("Ausführen: {}", ls_cmd);
    let output = ls_cmd.output_string()?;
    println!("Ergebnis (Ausschnitt):\n{}\n", output.lines().take(5).collect::<Vec<_>>().join("\n"));

    // 2. Befehl mit Umgebungsvariablen und Shell-Quoting
    // Entspricht: CONF_FILE='my config.conf' sh -c 'echo $CONF_FILE'
    let env_cmd = LinuxCommand::new("sh")
        .env("CONF_FILE", "my config.conf")
        .opt("-", "c", "echo $CONF_FILE");

    println!("Ausführen: {}", env_cmd);
    let output = env_cmd.output_string()?;
    println!("Ergebnis: {}\n", output.trim());

    // 3. Eine komplexe Pipe-Kette mit Shell-Argumenten
    // Entspricht: echo "Rust is awesome" | grep "awesome" | wc -w
    let pipe_chain = LinuxCommand::new("echo")
        .positional("Rust is awesome")
        .pipe(
            LinuxCommand::new("grep")
                .positional("awesome")
                .pipe(LinuxCommand::new("wc").flag("-", "w"))
        );

    println!("Ausführen: {}", pipe_chain);
    let count = pipe_chain.output_string()?;
    println!("Anzahl Wörter: {}\n", count.trim());

    // 4. Fehlerbehandlung: Ein Programm in der Mitte der Pipe schlägt fehl
    // Entspricht: echo "test" | false | cat
    println!("Test: Fehler in der Pipe-Mitte...");
    let fail_pipe = LinuxCommand::new("echo")
        .positional("test")
        .pipe(LinuxCommand::new("false"))
        .pipe(LinuxCommand::new("cat"));

    match fail_pipe.output_string() {
        Ok(_) => println!("Fehler: Sollte eigentlich fehlschlagen!"),
        Err(e) => println!("Erwarteter Fehler abgefangen: {}\n", e),
    }

    // 5. Letzter Befehl schlägt fehl (mit Stderr-Ausgabe)
    // Entspricht: ls /pfad/der/nicht/existiert
    println!("Test: Letzter Befehl schlägt fehl...");
    let error_cmd = LinuxCommand::new("ls")
        .positional("/non/existent/path");

    match error_cmd.output_string() {
        Ok(_) => println!("Fehler: Pfad sollte nicht existieren!"),
        Err(e) => println!("Erwarteter Fehler abgefangen: {}\n", e),
    }

    println!("--- Alle Tests abgeschlossen ---");
    Ok(())
}