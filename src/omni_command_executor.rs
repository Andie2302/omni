use crate::omni_command::OmniCommand;
use std::process::Command;

pub fn execute_dry_run(omni_command: &OmniCommand) {
    println!("--- Dry Run: {} ---", omni_command.name);
    println!("Full Command String: {}", omni_command);
    println!("Individual Arguments for OS:");

    let mut index = 0;
    for arg in &omni_command.args {
        for token in arg.to_os_args() {
            println!("  [{}] {:?}", index, token);
            index += 1;
        }
    }
    println!("-----------------------");
}

pub fn execute(omni_command: &OmniCommand) {
    println!("Executing: {}", omni_command);

    let mut process = Command::new(&omni_command.name);

    for arg in &omni_command.args {
        process.args(arg.to_os_args());
    }

    match process.status() {
        Ok(status) => println!("Finished with status: {}", status),
        Err(e) => eprintln!("Failed to execute command: {}", e),
    }
}