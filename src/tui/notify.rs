// Notify the TUI about changes in Fluster!
// This is the only place we lock the TUI state.

use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::{filesystem::filesystem_struct::USE_TUI, tui::{layout::FlusterTUI, tasks::{ProgressableTask, TaskHandle, TaskType}}};

// Global TUI state
lazy_static! {
    pub static ref TUI_MANAGER: Mutex<FlusterTUI<'static>> = Mutex::new(FlusterTUI::new());
}

// We only run some logic if the TUI is enabled.
// Plus this is a great excuse to learn macros!
// This just sticks a return into the function if the TUI is enabled.
macro_rules! skip_if_tui_disabled {
    () => {
        if !USE_TUI.get().expect("USE_TUI should be set") {
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

    /// Block(s) has been written to disk.
    pub(crate) fn block_written(amount: u16) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.disk_blocks_written += amount as u64;
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
    pub(crate) fn set_cache_hit_rate(rate: f64) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_hit_rate = rate;
    }

    /// Update the cache pressure
    pub(crate) fn set_cache_pressure(pressure: f64) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock().expect("Single thread, kinda.").state.cache_pressure = pressure;
    }

    //
    // Task
    //

    /// Start a new task.
    /// 
    /// You must keep the TaskHandle to be able to update this task.
    #[must_use] // Cant ignore the handle!
    pub(crate) fn start_task(task_type: TaskType, steps: u64) -> TaskHandle {
        // Return a dummy handle if TUI is disabled.
        if !USE_TUI.get().expect("USE_TUI should be set") {
            return TaskHandle::new();
        }

        // Create the task and make a new handle
        let new_task = ProgressableTask::new(task_type, steps);
        let handle: TaskHandle = TaskHandle::new();


        let state = &mut TUI_MANAGER.lock().expect("Single thread, kinda.").state;
        // If we already have a task, append it
        if let Some(ref mut task) = state.task {
            task.add_sub_task(new_task);
            return handle;
        }
        // Currently don't have any tasks, add it directly.
        state.task = Some(new_task);
        handle
    }

    /// Complete one step of a task
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn complete_task_step(_handle: &TaskHandle) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock()
        .expect("Single thread, kinda.")
        .state
        .task.as_mut()
        .expect("There shouldn't be any handles if we have no tasks.")
        .finish_steps(1);
    }

    /// Complete multiple steps of a task.
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn complete_multiple_task_steps(_handle: &TaskHandle, steps: u64) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock()
        .expect("Single thread, kinda.")
        .state
        .task.as_mut()
        .expect("There shouldn't be any handles if we have no tasks.")
        .finish_steps(steps);
    }

    /// Add more steps to the current task.
    ///
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn add_steps_to_task(_handle: &TaskHandle, steps: u64) {
        skip_if_tui_disabled!();
        TUI_MANAGER.lock()
        .expect("Single thread, kinda.")
        .state
        .task.as_mut()
        .expect("There shouldn't be any handles if we have no tasks.")
        .add_work(steps);
    }

    /// Finish a task.
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn finish_task(mut handle: TaskHandle) {
        skip_if_tui_disabled!();
        let stored_task = &mut TUI_MANAGER.lock().expect("Single thread, kinda.")
        .state
        .task;

        *stored_task = stored_task.take().expect("If a handle exists, so does a task.").finish_task();

        // Update and drop handle by letting it fall out of scope.
        handle.task_was_finished_or_canceled = true;
    }

    /// Cancel a task.
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn cancel_task(mut handle: TaskHandle) {
        skip_if_tui_disabled!();
        let stored_task = &mut TUI_MANAGER.lock().expect("Single thread, kinda.")
        .state
        .task;

        *stored_task = stored_task.take().expect("If a handle exists, so does a task.").cancel_task();

        // Update and drop handle by letting it fall out of scope.
        handle.task_was_finished_or_canceled = true;
    }

    /// Forcibly cancel a task without a handle.
    /// Only used for dropping
    pub(super) fn force_cancel_task() {
        skip_if_tui_disabled!();
        let stored_task = &mut TUI_MANAGER.lock().expect("Single thread, kinda.")
        .state
        .task;

        // Only try to cancel if there's actually a task
        if let Some(task) = stored_task.take() {
            *stored_task = task.cancel_task();
        }
    }
    
}




