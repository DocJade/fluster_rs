// Gotta talk to people sometimes.

use std::sync::{mpsc, Mutex};

use lazy_static::lazy_static;

// We need channels to talk through
lazy_static! {
    // filesystem sends, the TUI receives
    static ref TUI_REQUEST_SENDER: Mutex<mpsc::Sender<TuiPrompt>> = {
        let (tx, _) = mpsc::channel();
        Mutex::new(tx)
    };

    // TUI uses this for receiving
    static ref TUI_REQUEST_RECEIVER: Mutex<mpsc::Receiver<TuiPrompt>> = {
        // This is a bit of a trick to share the receiver from the sender.
        let (tx, rx) = mpsc::channel();
        *TUI_REQUEST_SENDER.lock().expect("Man i dont even know if this is safe, shame on me") = tx;
        Mutex::new(rx)
    };
}

// Helper to get the sender
pub(crate) fn get_tui_sender() -> mpsc::Sender<TuiPrompt> {
    let _ = &*TUI_REQUEST_RECEIVER; 
    TUI_REQUEST_SENDER.lock().expect("idk anymore").clone()
}



pub(super) struct TuiPrompt {
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
