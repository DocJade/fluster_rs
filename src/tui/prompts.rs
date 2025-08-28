// Gotta talk to people sometimes.

use crate::{filesystem::filesystem_struct::USE_TUI, tui::notify::TUI_MANAGER};

pub(crate) struct TuiPrompt {
    /// Title of the prompt
    pub(super) title: String,
    /// What the prompt is telling the user
    pub(super) content: String,
    /// Do we expect input? If so, where to?
    pub(super) response: Option<oneshot::Sender<String>>,
    /// Should the window flash to get the user's attention?
    pub(super) flash: bool,
}




// if the TUI is disabled, we still need to be able to prompt for input.
impl TuiPrompt {
    /// Make a new prompt for pressing enter.
    /// 
    /// This will block until the user presses enter.
    pub(crate) fn prompt_enter(title: String, content: String, flash: bool) {
        // Assemble and start the prompt.
        let prompt = TuiPrompt {
            title,
            content,
            response: None,
            flash,
        };

        if !USE_TUI.get().expect("USE_TUI should be set") {
            return legacy_prompt_enter(prompt);
        }

        // Run the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.try_lock() {
                lock.user_prompt = Some(prompt);
                break
            }
        }

        // Now we wait for the prompt to be gone (ie the user finished it)
        loop {
            {
                // Another block so we dont hold onto the lock
                if let Ok(lock) = TUI_MANAGER.try_lock() {
                    if lock.user_prompt.is_none() {
                        break
                    }
                }
            }
            // Still waiting. Stall for a bit.
            std::thread::sleep(std::time::Duration::from_millis(32));
        } 
        // All done.
    }

    /// Make a new prompt for text input.
    /// 
    /// This will block until the user responds.
    pub(crate) fn prompt_input(title: String, content: String, flash: bool) -> String {
        // Get the channel for communicating the result of the prompt
        let (response_tx, response_rx) = oneshot::channel();


        // Assemble and start the prompt.
        let prompt = TuiPrompt {
            title,
            content,
            response: Some(response_tx),
            flash,
        };

        if !USE_TUI.get().expect("USE_TUI should be set") {
            return legacy_prompt_input(prompt);
        }

        // Run the prompt
        loop {
            if let Ok(mut lock) = TUI_MANAGER.try_lock() {
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