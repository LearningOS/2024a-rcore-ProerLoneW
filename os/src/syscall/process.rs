//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{exit_current_and_run_next, suspend_current_and_run_next, TaskStatus},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
// pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
//     trace!("kernel: sys_task_info");
//     -1
// }
use crate::task::TASK_MANAGER;
// use crate::config::CLOCK_FREQ;
use crate::timer::get_time_ms;

pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    if ti.is_null() {
        return -1;
    }

    let inner = TASK_MANAGER.inner_exclusive_access();
    let current_task_id = inner.current_task;
    let current_task = &inner.tasks[current_task_id];

    // 计算运行时间
    let run_time = get_time_ms() - current_task.start_time;

    // 获取系统调用计数
    let syscall_times = current_task.syscall_count.clone();

    // 填充 TaskInfo
    unsafe {
        *ti = TaskInfo {
            status: current_task.task_status,
            syscall_times,
            time: run_time,
        };
    }

    0
}
