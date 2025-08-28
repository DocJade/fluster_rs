// Updating whats on screen.
// Would be a bad idea to do it _too_ often, so we need to gate it
// to a maximum refresh speed.
//
// Additionally, we don't wanna update it ourselves with another thread, we should
// manually pick when we update by having actions within the filesystem call the update
// function. This should also prevent issues with locking.

use std::time::Instant;

use crate::tui::{layout::FlusterTUI, state::FlusterTUIState, tasks::ProgressableTask};

impl FlusterTUI {
    /// Complete a step in the currently in-progress task.
    pub(super) fn task_step(&mut self) {
        self.state.task.as_mut().expect("Can't progress without a task!").finish_step();
    }

    /// Add more work to the current task
    pub(super) fn task_add_work(&mut self, steps_to_add: u64) {
        self.state.task.as_mut().expect("Can't add work without a task!").add_work(steps_to_add);
    }

    /// Finish the task we are currently working on.
    pub(super) fn task_finish(&mut self) {
        // We need to take the value since finish_task consumes it.
        self.state.task = self.state.task.take().expect("Must have a task to finish!").finish_task();
    }

    /// Cancel the task we were working on.
    pub(super) fn task_cancel(&mut self) {
        // We need to take the value since cancel_task consumes it.
        self.state.task = self.state.task.take().expect("Must have a task to cancel!").cancel_task();
    }

    /// Create a new task to work on.
    pub(super) fn new_task(&mut self, new_task: ProgressableTask) {
        // If we already have a task, append it
        if let Some(task) = self.state.task.as_mut() {
            task.add_sub_task(new_task);
            return
        }
        // Currently don't have any tasks, add it directly.
        self.state.task = Some(new_task);
    }

    /// Brand new state, only for initialization.
    pub(super) fn new() -> Self {
        Self {
            state: FlusterTUIState::new(),
            last_update: Instant::now(),
        }
    }
}