// Keeping track of what we're working on

use std::{fmt, path::Path, time::Instant};

use crate::tui::notify::NotifyTui;


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
#[derive(Debug)]
pub(crate) struct ProgressableTask {
    /// What the task is
    task: TaskInfo,
    /// A sub task, if any.
    sub_task: Option<Box<ProgressableTask>>
}

/// Task information is stored in a second struct for ease of use, since
/// everything besides sub-tasks can be Copy-ed.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub(crate) enum TaskType {
    DiskWriteBlock,
    DiskWriteLarge,
    WaitingForDriveSpinUp,
    DiskReadBlock,
    /// Includes the name of the file.
    FilesystemReadFile(String), // man strings are annoying, no more copy
    /// Includes the name of the file.
    FilesystemWriteFile(String),
    /// Includes the name of the file.
    FilesystemOpenFile(String),
    /// Includes the name of the file.
    FilesystemTruncateFile(String),
    /// Includes name of new file duh
    FilesystemCreateFile(String),
    /// Includes the name of the new directory.
    FilesystemMakeDirectory(String),
    /// Includes the name of directory to list.
    FilesystemReadDirectory(String),
    /// Includes the name of the directory that is about to be removed.
    FilesystemRemoveDirectory(String),
    /// Includes the name of the file that is kill.
    FilesystemDeleteFile(String),
    /// Includes the name of the file / folder.
    GetMetadata(String),
    GetSize,
    WipeDisk,
    FlushCurrentDisk,
    FlushTier,
    /// Includes number of requested blocks.
    PoolAllocateBlocks(u16),
    /// Includes number of blocks being deallocated.
    DiskDeallocateBlocks(u16),
    CreateNewDisk,
    WriteCRC,
    /// Includes the name of the directory we're trying to open
    ChangingDirectory(String),
    ListingDirectory,
    /// Includes the name of the item we're looking for
    FindItemInDirectory(String),
    CreateDirectoryItem,
}

/// When we start a task, we are promising to finish it. We need a way to know
/// if the task never finished. Thus we hand out a little struct that you need to
/// hold onto. If the struct gets dropped, we assume the lowest task in the chain
/// was canceled, or has finished.
/// 
/// DO NOT implement clone or copy.
pub(crate) struct TaskHandle {
    /// This is set if the the task this handle was dispatched for
    /// completed _or_ was canceled, indicating that cleanup is not needed.
    /// 
    /// Defaults to false
    pub(super) task_was_finished_or_canceled: bool,
}



//
// Implementations
//

impl Drop for TaskHandle {
    fn drop(&mut self) {
        // Are we done?
        if self.task_was_finished_or_canceled {
            // Cool! Don't need to do anything.
        } else {
            // The task was never finished or canceled, we must cancel it ourselves
            // We cant actual use ourselves here, since we need to pass in an owned
            // value that will be dropped. So we just make a new one. weird, i know
            NotifyTui::force_cancel_task()
        }
    }
}

impl TaskHandle {
    pub(super) fn new() -> Self {
        Self {
            task_was_finished_or_canceled: false,
        }
    }
}




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
            TaskType::WaitingForDriveSpinUp => "Waiting for floppy drive to spin up...".to_string(),
            TaskType::DiskWriteBlock => "Writing a block...".to_string(),
            TaskType::DiskReadBlock => "Reading a block...".to_string(),
            TaskType::WipeDisk => "Wiping disk...".to_string(),
            TaskType::DiskWriteLarge => "Writing several blocks...".to_string(),
            TaskType::CreateNewDisk => "Creating new disk...".to_string(),
            TaskType::WriteCRC => "Writing CRC to block...".to_string(),
            TaskType::GetSize => "Getting the size of an item...".to_string(),
            TaskType::ListingDirectory => "Listing a directory...".to_string(),
            TaskType::CreateDirectoryItem => "Creating new directory item...".to_string(),
            TaskType::FlushCurrentDisk => "Flushing current disk...".to_string(),
            TaskType::FlushTier => "Flushing a tier of cache...".to_string(),
            TaskType::FilesystemReadFile(name) => {
                format!("Reading from file \"{name}\"...")
            },
            TaskType::FilesystemWriteFile(name) => {
                format!("Writing to file \"{name}\"...")
            },
            TaskType::FilesystemTruncateFile(name) => {
                format!("Truncating \"{name}\"...")
            },
            TaskType::PoolAllocateBlocks(number) => {
                format!("Allocating {number} blocks across disk pool...")
            },
            TaskType::DiskDeallocateBlocks(number) => {
                format!("Freeing {number} blocks from current disk...")
            },
            TaskType::GetMetadata(name) => {
                format!("Getting {name}'s metadata...")
            },
            TaskType::ChangingDirectory(name) => {
                format!("Trying to open directory {name}...")
            },
            TaskType::FindItemInDirectory(name) => {
                format!("Looking for {name} in current directory...")
            },
            TaskType::FilesystemMakeDirectory(name) => {
                format!("Making new directory {name}...")
            },
            TaskType::FilesystemOpenFile(name) => {
                format!("Opening file {name}...")
            },
            TaskType::FilesystemCreateFile(name) => {
                format!("Creating file {name}...")
            },
            TaskType::FilesystemReadDirectory(name) => {
                format!("Reading directory {name}...")
            },
            TaskType::FilesystemRemoveDirectory(name) => {
                format!("Removing directory {name}...")
            },
            TaskType::FilesystemDeleteFile(name) => {
                format!("Deleting file {name}...")
            },
        }
    }

    /// Get a float for how finished the task is
    pub(super) fn progress(&self) -> f64 {
        self.steps_finished as f64 / self.steps as f64
    }

    /// Returns a string `[hh:mm:ss]` of how long the task has been running
    pub(super) fn time_passed(&self) -> String {
        let elapsed_seconds = self.start_time.elapsed().as_secs();
        format!("[{:0>2}:{:0>2}:{:0>2}]", elapsed_seconds/60*60, elapsed_seconds/60, elapsed_seconds%60)
    }
    
    /// Returns a string `[hh:mm:ss]` which guesstimates how long the task is going to take
    /// 
    /// Barely useful, but fun!
    pub(super) fn time_remaining(&self) -> String {
        let elapsed_seconds = self.start_time.elapsed().as_secs();
        // Now based on how far we've come, estimate how much longer it'll take
        let mut estimated_seconds = if self.steps_finished > 0 {
            (elapsed_seconds as f64 * (self.steps as f64 / self.steps_finished as f64)) as u64
        } else {
            // div zero moment, just return all nines lmao
            return "[99:99:99]".to_string();
        };
        estimated_seconds = estimated_seconds.saturating_sub(elapsed_seconds);
        format!("[{:0>2}:{:0>2}:{:0>2}]", estimated_seconds/60*60, estimated_seconds/60, estimated_seconds%60)
    }
}

impl ProgressableTask {
    /// Create a new task.
    /// 
    /// New tasks cannot have a sub task pre-attached.
    pub(super) fn new(task_type: TaskType, steps: u64) -> ProgressableTask {
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
    pub(super) fn finish_steps(&mut self, steps: u64) {
        // Recurse if there are sub-tasks
        if let Some(sub_task) = &mut self.sub_task {
            return sub_task.finish_steps(steps)
        }

        self.task.steps_finished += steps;
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
        assert!(self.task.steps == self.task.steps_finished, "Task was not finished! {:#?}", self.task);

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