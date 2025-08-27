// how da tui looks.

use std::time::Instant;

use crate::tui::state::FlusterTUIState;

/// The TUI interface of fluster. Call methods on this to update the interface as often as you'd like.
pub(crate) struct FlusterTUI {
    /// The actual internal state
    pub(super) state: FlusterTUIState,
    /// The last time the interface was updated
    pub(super) last_update: Instant
}