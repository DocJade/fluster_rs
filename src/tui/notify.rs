// Notify the TUI about changes in Fluster!
// This is the only place we lock the TUI state.

use std::sync::Mutex;

use lazy_static::lazy_static;
use log::error;

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
        if let Some(got) = USE_TUI.get() {
            if !got {
                // TUI is not enabled.
                return
            }
        } else {
            // The flag is not currently set, we'll just do all of the logic behind the scenes just
            // in case.

        }
    };
}

// Since we expect things to be single threaded when interacting with the TUI, we have a lot of
// locks and expects, thus we will just abstract that out. If the lock fails, the task state would become
// desynced, which would almost guarantee a crash. Thus we will exit out if that happens.
//
// Karen, because it gets the manager
macro_rules! karen {
    () => {
        if let Ok(got) = TUI_MANAGER.lock() {
            got
        } else {
            // Manager is out, which mean's its poisoned due to a panic or such.
            // Not great. I really doubt we could recover from that.
            error!("Couldn't get the TUI manager, giving up!");
            error!("{}", std::backtrace::Backtrace::force_capture());
            panic!("TUI manager is poisoned!");
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

    /// A _real_ disk swap has occurred, you must provide the disk that is now in the drive.
    pub(crate) fn disk_swapped(new_disk: u16) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.disk_swap_count += 1;
        manager.state.current_disk_in_drive = new_disk;
    }

    /// A block, or multiple blocks have been read from disk.
    pub(crate) fn block_read(number: u16) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.disk_blocks_read += number as u64;
    }

    /// Block(s) has been written to disk.
    pub(crate) fn block_written(amount: u16) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.disk_blocks_written += amount as u64;
    }

    //
    // Cache
    //

    /// The cache saved a swap.
    pub(crate) fn swap_saved() {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.cache_swaps_saved += 1;
    }

    /// A tier of the cache was flushed to disk.
    pub(crate) fn cache_flushed() {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.cache_flushes += 1;
    }

    /// A read was cached instead of read from disk.
    pub(crate) fn read_cached() {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.cache_blocks_read += 1;
    }

    /// A write was cached instead of written to disk.
    pub(crate) fn write_cached() {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.cache_blocks_written += 1;
    }

    /// Update the cache hit-rate
    pub(crate) fn set_cache_hit_rate(rate: f64) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.cache_hit_rate = rate;
    }

    /// Update the cache pressure
    pub(crate) fn set_cache_pressure(pressure: f64) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        manager.state.cache_pressure = pressure;
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
        if let Some(flag) = USE_TUI.get() {
            if !flag {
                return TaskHandle::new();
            }
        } else {
            // USE_TUI is not set, this really should be set already...
            // If we give out a fake handle:
            // - Caller cancels/finishes a task, but now USE_TUI is set, causing
            //    either nothing to happen, or canceling an unrelated task. BAD!
            // If we give out a TUI handle in non-TUI mode:
            // - Handle would be given out, added to the TUI state, now USE_TUI is set,
            //    if its set to true, all is good. If its false, still okay, since it just will chill
            //    in the tui manager without ever being cleaned up. Which is fine.

            // Thus we will give out a handle that is already marked as finished, so when the deconstruction on it runs,
            // It'll just drop silently. Hopefully? In theory we can only give out one handle at a time, so if this is the
            // only handle that's out, and the flag is not set, chances are this is the first handle _ever_ so ignoring it is fine,
            // because the TUI would just try to clean up nothing, which should finish.

            // Also before adding this if let some, there was an expect here, and i never saw that get hit, so chances are
            // this will never run anyways.

            let mut pre_finished = TaskHandle::new();
            pre_finished.task_was_finished_or_canceled = true;
            return pre_finished;
        }

        // Create the task and make a new handle
        let new_task = ProgressableTask::new(task_type, steps);
        let handle: TaskHandle = TaskHandle::new();

        let mut manager = karen!();
        let state = &mut manager.state;
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
        let mut manager = karen!();
        if let Some(task) = manager.state.task.as_mut() {
            task.finish_steps(1);
        } else {
            // No task to work on! Out of sync!
            // Cooked for sure.
            error!("{}", std::backtrace::Backtrace::force_capture());
            panic!("Task state desync! Expected task, got none!");
        }
    }

    /// Complete multiple steps of a task.
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn complete_multiple_task_steps(_handle: &TaskHandle, steps: u64) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        if let Some(task) = manager.state.task.as_mut() {
            task.finish_steps(steps);
        } else {
            // No task to work on! Out of sync!
            // Cooked for sure.
            error!("{}", std::backtrace::Backtrace::force_capture());
            panic!("Task state desync! Expected task, got none!");
        }
    }

    /// Add more steps to the current task.
    ///
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn add_steps_to_task(_handle: &TaskHandle, steps: u64) {
                skip_if_tui_disabled!();
        let mut manager = karen!();
        if let Some(task) = manager.state.task.as_mut() {
            task.add_work(steps);
        } else {
            // No task to work on! Out of sync!
            // Cooked for sure.
            error!("{}", std::backtrace::Backtrace::force_capture());
            panic!("Task state desync! Expected task, got none!");
        }
    }

    /// Finish a task.
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn finish_task(mut handle: TaskHandle) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        let stored_task = &mut manager
        .state
        .task;

        if let Some(took) = stored_task.take() {
            *stored_task = took.finish_task();
        } else {
            // We have a handle, but no task?
            // I mean, there's nothing to finish, but removing nothing shouldn't be an issue.
            // Good enough?
        }

        // Update and drop handle by letting it fall out of scope.
        handle.task_was_finished_or_canceled = true;
    }

    /// Cancel a task.
    /// 
    /// Handle required to ensure you actually have a task you're working on
    pub(crate) fn cancel_task(mut handle: TaskHandle) {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        let stored_task = &mut manager
        .state
        .task;

        if let Some(took) = stored_task.take() {
            *stored_task = took.cancel_task();
        } else {
            // We have a handle, but no task?
            // I mean, there's nothing to finish, but removing nothing shouldn't be an issue.
            // Good enough?
        }

        // Update and drop handle by letting it fall out of scope.
        handle.task_was_finished_or_canceled = true;
    }

    /// Forcibly cancel a task without a handle.
    /// Only used for dropping
    pub(super) fn force_cancel_task() {
        skip_if_tui_disabled!();
        let mut manager = karen!();
        let stored_task = &mut manager
        .state
        .task;

        // Only try to cancel if there's actually a task
        if let Some(task) = stored_task.take() {
            *stored_task = task.cancel_task();
        }
    }
    
}




