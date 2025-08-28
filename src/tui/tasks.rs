// Keeping track of what we're working on

use std::{fmt, path::Path, time::Instant};


/// Progress of events.
/// 
/// Updating an event's progress is an indirect action, you can say you've completed a "steps" of a
/// task, or add more "steps" to the task. Thus you don't have to keep track of the percentages yourself,
/// just if you need to add more work to your task, or if you've completed some of it.
/// 
/// Tasks can have sub-tasks.
/// 
/// Keep in mind that steps only increment, you cannot go backwards.
/// 
/// All actions on a ProgressableTask implicitly apply to the final task in the chain of
/// sub-tasks. IE, if you have a->b->c, calling finish_step() will affect c.
pub(crate) struct ProgressableTask {
    /// What the task is
    task: TaskInfo,
    /// A sub task, if any.
    sub_task: Option<Box<ProgressableTask>>
}

/// Task information is stored in a second struct for ease of use, since
/// everything besides sub-tasks can be Copy-ed.
#[derive(Clone)]
pub(super) struct TaskInfo {
    /// The type of task being performed.
    task_type: TaskType,
    /// How many "steps" of the task are needed to finish this task.
    steps: u64,
    /// How many "steps" have been completed so far.
    steps_finished: u64,
    /// When this task was started
    start_time: Instant,
}

/// Every kind of task that can indicate its progress.
#[derive(Clone)]
pub(crate) enum TaskType {
    DiskWriteBlock,
    DiskReadBlock,
    /// Includes the name of the file.
    FilesystemReadFile(String), // man strings are annoying, no more copy
    /// Includes the name of the file.
    FilesystemWriteFile(String),
    /// Includes the name of the file.
    FilesystemTruncateFile(String),
}



//
// Implementations
//

impl TaskInfo {
    // yeah
    fn new(task_type: TaskType, steps: u64) -> TaskInfo {
        Self {
            task_type,
            steps,
            steps_finished: 0,
            start_time: Instant::now(),
        }
    }

    /// Get the string name of this task
    pub(super) fn name(&self) -> String {
        match &self.task_type {
            TaskType::DiskWriteBlock => "Writing a block...".to_string(),
            TaskType::DiskReadBlock => "Reading a block...".to_string(),
            TaskType::FilesystemReadFile(name) => {
                        format!("Reading from file \"{name}\"...")
                    },
            TaskType::FilesystemWriteFile(name) => {
                        format!("Writing to file \"{name}\"...")
                    },
            TaskType::FilesystemTruncateFile(name) => {
                        format!("Truncating \"{name}\"...")
                    },
        }
    }

    /// Get a float for how finished the task is
    pub(super) fn progress(&self) -> f64 {
        self.steps_finished as f64 / self.steps as f64
    }
}

impl ProgressableTask {
    /// Create a new task.
    /// 
    /// New tasks cannot have a sub task pre-attached.
    pub(crate) fn new(task_type: TaskType, steps: u64) -> ProgressableTask {
        ProgressableTask {
            task: TaskInfo::new(task_type, steps),
            sub_task: None,
        }
    }

    /// Add more steps to a currently in-progress task.
    /// 
    /// Updates how many steps _need_ to be taken, not how many _have_ been taken.
    pub(super) fn add_work(&mut self, steps_to_add: u64) {
        // Recurse if there are sub-tasks
        if let Some(sub_task) = &mut self.sub_task {
            return sub_task.add_work(steps_to_add)
        }

        self.task.steps += steps_to_add;
    }

    /// Indicate that a step has been completed.
    /// 
    /// You can only finish one step at a time.
    pub(super) fn finish_step(&mut self) {
        // Recurse if there are sub-tasks
        if let Some(sub_task) = &mut self.sub_task {
            return sub_task.finish_step()
        }

        self.task.steps_finished += 1;
        // You cannot finish more steps than you need to complete.
        assert!(self.task.steps_finished <= self.task.steps);
    }

    /// Add a sub-task.
    /// 
    /// New task will be added to the last task in the chain.
    pub(super) fn add_sub_task(&mut self, new_sub_task: ProgressableTask) {
        // Recurse if there are sub-tasks
        if let Some(sub_task) = &mut self.sub_task {
            return sub_task.add_sub_task(new_sub_task)
        }

        self.sub_task = Some(Box::new(new_sub_task));
    }

    /// Finishes the task (or subtask) currently in progress.
    /// 
    /// Does not return the finished task, it only removes it.
    /// 
    /// For a task to end, it must have a step count equal to its finished step count. IE
    /// you cannot finish a task without completing all of the work you promised you would complete.
    /// 
    /// If you need to end a task due to a failure, you need to use cancel_task()
    /// 
    /// Will return None if this is the final task in the chain.
    pub(super) fn finish_task(mut self) -> Option<ProgressableTask> {
        // Recurse if there are sub-tasks
        if let Some(sub_task) = self.sub_task {
            let new_subtask = sub_task.finish_task();
            self.sub_task = new_subtask.map(Box::new);
            return Some(self)
        }
        
        // This is the final task in the chain.
        assert!(self.task.steps == self.task.steps_finished, "Task was not finished!");

        // Remove the task.
        None
    }

    /// Cancels an in-progress task (or subtask).
    /// 
    /// This ignores the requirement for tasks to have all of their steps completed.
    /// 
    /// This is an explicit function, as you should do proper handling for canceling tasks.
    pub(super) fn cancel_task(mut self) -> Option<ProgressableTask> {
        // Recurse if there are sub-tasks
        if let Some(sub_task) = self.sub_task {
            let new_subtask = sub_task.cancel_task();
            self.sub_task = new_subtask.map(Box::new);
            return Some(self)
        }
        // This is the last task, remove it.
        None
    }

    /// Extract a Vec<TaskInfo> from this task.
    /// 
    /// The head task is the first in the vec.
    pub(super) fn get_tasks_info(&self) -> Vec<TaskInfo> {
        // Recurse downwards, adding a copy of each task info as we go down.
        let mut tasks: Vec<TaskInfo> = Vec::new();
        tasks.push(self.task.clone());

        let mut sub = &self.sub_task;
        while let Some(new_sub) = sub.as_deref() {
            // Get the task out
            tasks.push(new_sub.task.clone());
            // go deeper
            sub = &new_sub.sub_task;
        }

        // All tasks have been collected.
        tasks
    }
}