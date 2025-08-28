// Gotta talk to people sometimes.

use ratatui::style::{Style, Stylize};
use tui_textarea::TextArea;

use crate::{filesystem::filesystem_struct::USE_TUI, tui::notify::TUI_MANAGER};

pub(crate) struct TuiPrompt<'a> {
    /// Title of the prompt
    pub(super) title: String,
    /// What the prompt is telling the user
    pub(super) content: String,
    /// Do we expect to get a string back from this prompt?
    pub(super) expects_string: bool,
    /// Where we send the response to. Even if the prompt
    /// doesn't require a response, we still use the oneshot to more
    /// easily block the caller of the prompt.
    /// 
    /// I'm calling this a callback because i feel like it.
    pub(super) callback: oneshot::Sender<String>,
    /// Should the window flash to get the user's attention?
    pub(super) flash: bool,
    /// The persistent text entry field, persists between
    /// frames so we don't have to extract input handling out
    /// to main.rs
    pub(super) text_area: tui_textarea::TextArea<'a>
}




// if the TUI is disabled, we still need to be able to prompt for input.
impl TuiPrompt<'_> {
    /// Make a new prompt for pressing enter.
    /// 
    /// This will block until the user presses enter.
    pub(crate) fn prompt_enter(title: String, content: String, flash: bool) {
        // We need the channel even if we arent getting a string back, since we wanna
        // block the caller thread without having to spin in a loop lockin stuff.
        let (response_tx, response_rx) = oneshot::channel();
        // Assemble and start the prompt.
        let prompt = TuiPrompt {
            title,
            content,
            expects_string: false,
            callback: response_tx,
            flash,
            text_area: TextArea::default() // Not actually used.
        };

        if !USE_TUI.get().expect("USE_TUI should be set") {
            return legacy_prompt_enter(prompt);
        }

        // Run the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.lock() {
                lock.user_prompt = Some(prompt);
                break
            }
        }

        // Wait for prompt to close
        let _ = response_rx.recv();

        // All done.
    }

    /// Make a new prompt for text input.
    /// 
    /// This will block until the user responds.
    pub(crate) fn prompt_input(title: String, content: String, flash: bool) -> String {
        // Get the channel for communicating the result of the prompt
        let (response_tx, response_rx) = oneshot::channel();


        // Assemble and start the prompt.
        // green text box
        let mut text_area = TextArea::default();
        text_area.set_style(Style::reset().on_black().green());
        let prompt = TuiPrompt {
            title,
            content,
            expects_string: true,
            callback: response_tx,
            flash,
            text_area,
        };

        if !USE_TUI.get().expect("USE_TUI should be set") {
            return legacy_prompt_input(prompt);
        }

        // Run the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.lock() {
                lock.user_prompt = Some(prompt);
                break
            }
        }

        // Wait for a response, and return it.
        response_rx.recv().expect("Sender should send.")
    }
}


// Prompt without the TUI
fn legacy_prompt_enter(prompt: TuiPrompt) {
    let _ = rprompt::prompt_reply(format!("[{}]: {}", prompt.title, prompt.content));
}

fn legacy_prompt_input(prompt: TuiPrompt) -> String {
    rprompt::prompt_reply(format!("[{}]: {}", prompt.title, prompt.content)).expect("stdin should not fail.")
}