//! Types related to task management

use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The task start time
    pub start_time: usize,
    /// The task syscall times
    pub syscall_count: [u32; MAX_SYSCALL_NUM],
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}

impl TaskControlBlock {
    /// init time
    pub fn new() -> Self {
        Self {
            task_status: TaskStatus::UnInit,
            task_cx: TaskContext::zero_init(),
            start_time: 0,
            syscall_count: [0; 500],
        }
    }

    /// update syscall time
    pub fn increment_syscall_count(&mut self, syscall_id: usize) {
        if syscall_id < MAX_SYSCALL_NUM {
            self.syscall_count[syscall_id] += 1;
        }
    }
}
