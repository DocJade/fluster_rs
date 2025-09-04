// The struct that holds everything needed to render the tui

use std::time::Instant;

use crate::tui::{
    layout::FlusterTUI,
    state::FlusterTUIState
};

impl FlusterTUI<'_> {
    /// Brand new state, only for initialization.
    pub(super) fn new() -> Self {
        Self {
            state: FlusterTUIState::new(),
            last_update: Instant::now(),
            started: Instant::now(),
            user_prompt: None,
        }
    }
}