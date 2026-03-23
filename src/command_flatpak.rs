use crate::omni_command::{OmniCommand, OmniCommandArg};
pub struct FlatpakCommand;
impl FlatpakCommand {
    fn base(subcommand: &str) -> OmniCommand {
        OmniCommand::new("flatpak")
            .with_arg(OmniCommandArg::new(subcommand))
            .with_arg(OmniCommandArg::new("--assumeyes"))
            .with_arg(OmniCommandArg::new("--noninteractive"))
    }

    pub fn install(remote: &str, app_id: &str) -> OmniCommand {
        Self::base("install")
            .with_arg(OmniCommandArg::new("--or-update"))
            .with_arg(OmniCommandArg::new(remote))
            .with_arg(OmniCommandArg::new(app_id))
    }

    pub fn update(app_id: &str) -> OmniCommand {
        Self::base("update")
            .with_arg(OmniCommandArg::new(app_id))
    }

    pub fn remove(app_id: &str) -> OmniCommand {
        Self::base("remove")
            .with_arg(OmniCommandArg::new(app_id))
    }

}