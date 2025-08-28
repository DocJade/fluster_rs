// Notify the TUI about changes in Fluster!
// This is the only place we lock the TUI state.

use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::{filesystem::filesystem_struct::USE_TUI, tui::{layout::FlusterTUI, tasks::ProgressableTask}};

// Global TUI state
lazy_static! {
    pub static ref TUI_MANAGER: Mutex<FlusterTUI> = Mutex::new(FlusterTUI::new());
}

// We only run some logic if the TUI is enabled.
// Plus this is a great excuse to learn macros!
// This just sticks a return into the function if the TUI is enabled.
macro_rules! skip_if_tui_disabled {
    () => {
        if !USE_TUI.get().expect("USE_TUI should be enabled.") {
            return
        }
    };
}


// Now we have a bunch of public functions for updating the TUI.

// Its on a struct, just because I like how that looks.
pub(crate) struct NotifyTui {
    // dummy
}

impl NotifyTui {
    //
    // Disk
    //

    /// A _real_ disk swap has occurred.
    pub(crate) fn disk_swapped() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.disk_swap_count += 1;
    }

    /// A block has been read from disk.
    pub(crate) fn block_read() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.disk_blocks_read += 1;
    }

    /// A block has been written to disk.
    pub(crate) fn block_written() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.disk_blocks_written += 1;
    }

    //
    // Cache
    //

    /// The cache saved a swap.
    pub(crate) fn swap_saved() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_swaps_saved += 1;
    }

    /// A tier of the cache was flushed to disk.
    pub(crate) fn cache_flushed() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_flushes += 1;
    }

    /// A read was cached instead of read from disk.
    pub(crate) fn read_cached() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_blocks_read += 1;
    }

    /// A write was cached instead of written to disk.
    pub(crate) fn write_cached() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_blocks_written += 1;
    }

    /// Update the cache hit-rate
    pub(super) fn set_cache_hit_rate(rate: f32) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_hit_rate = rate;
    }

    //
    // Task
    //

    /// Start a new task.
    pub(super) fn start_task(new: ProgressableTask) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").new_task(new);
    }

    /// Complete a step of a task
    pub(super) fn complete_task_step() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").task_step();
    }

    /// Add more steps to the current task.
    pub(super) fn add_steps_to_task(steps: u64) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").task_add_work(steps);
    }

    /// Finish a task.
    pub(super) fn finish_task() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").task_finish();
    }

    /// Cancel a task.
    pub(super) fn cancel_task() {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").task_cancel();
    }
    
}




