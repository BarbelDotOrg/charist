use crate::app::{CharistApp, Modal};
use crate::config::CopyIncludeReferencePolicy;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Toggle,
    SetCopyReferencePolicy(CopyIncludeReferencePolicy),
    ToggleCopyVerseNumbers(bool),
    ToggleCopyDelimiter(bool),
}

impl CharistApp {
    pub(crate) fn update_settings(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::Toggle => {
                self.modal = match self.modal {
                    Some(Modal::Settings) => None,
                    _ => Some(Modal::Settings),
                };
            }
            SettingsMessage::SetCopyReferencePolicy(policy) => {
                self.config.copy_includes_reference_policy = policy;
            }
            SettingsMessage::ToggleCopyVerseNumbers(enabled) => {
                self.config.copy_includes_verse_numbers = enabled;
            }
            SettingsMessage::ToggleCopyDelimiter(enabled) => {
                self.config.copy_delimitate_with_newline = enabled;
            }
        }
        // Settings should feel persistent immediately, unlike book/chapter/
        // bookmark state which only saves on window close.
        self.save_config();
    }
}
