#![allow(dead_code)]
mod command_flatpak;
mod omni_command_executor;
mod omni_command;

use crate::omni_command::{OmniCommand, OmniCommandArg};
use crate::command_flatpak::FlatpakCommand;
use crate::omni_command_executor::execute_dry_run;

fn main() {
    let ffmpeg_test = OmniCommand::new("ffmpeg")
        .with_arg(OmniCommandArg::new("i").with_prefix("-").with_value("Urlaub Video.mp4"))
        .with_arg(OmniCommandArg::new("c:v").with_prefix("-").with_value("libx264"));
    let flatpak_test = FlatpakCommand::install("flathub", "org.mozilla.firefox");
    let separator_test = OmniCommand::new("mytool")
        .with_arg(OmniCommandArg::new("output")
            .with_prefix("--")
            .with_separator("=")
            .with_value("result.json"));
    let ls_test = OmniCommand::new("ls")
        .with_arg(OmniCommandArg::new("l").with_prefix("-"))
        .with_arg(OmniCommandArg::new("all").with_prefix("--"));
    execute_dry_run(&ffmpeg_test);
    execute_dry_run(&flatpak_test);
    execute_dry_run(&separator_test);
    execute_dry_run(&ls_test);
}