#![allow(dead_code)]
use std::sync::Arc;
use crate::omni_command_executor::OutputHandler;

mod omni_command_executor;
mod omni_command;
mod command_flatpak;

fn main() {
    // 1. Setup: Wir wollen die Ausgabe sammeln statt sie nur zu drucken
    let handler = Arc::new(omni_command_executor::CollectingHandler::new());

    // 2. Konfiguration: Live-Modus, aber Ausgabe "einfangen"
    let config = omni_command_executor::ExecutorConfig::new()
        .dry_run(false)
        .capture_output(true);

    let executor = omni_command_executor::OmniExecutor::new(config, Arc::clone(&handler) as Arc<dyn OutputHandler>);

    // 3. Einen Befehl erstellen (z.B. Flatpak Liste)
    let cmd = crate::omni_command::OmniCommand::new("flatpak")
        .with_arg(crate::omni_command::OmniCommandArg::new("list"));

    // 4. Ausführen
    let result = executor.execute(&cmd);

    if result.is_success() {
        // Jetzt können wir die gesammelten Daten verarbeiten!
        let output = handler.stdout();
        if output.contains("firefox") {
            println!("Firefox ist bereits installiert.");
        }
    } else {
        println!("Fehler: {}", result.status_message());
    }
}