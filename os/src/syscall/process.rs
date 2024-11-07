//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    },
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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
// pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
//     trace!("kernel: sys_get_time");
//     -1
// }
use crate::timer::{get_time_ms, get_time_us};
use crate::task::TASK_MANAGER;
use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    
    // 获取时间（假设用 get_time_sec_usec 函数来获取秒和微秒）
    let sec: usize= get_time_ms() / 1000;
    let usec = get_time_us();
    // 使用 translated_byte_buffer 处理跨页情况
    let mut buffers = translated_byte_buffer(current_user_token(), ts as *const u8, core::mem::size_of::<TimeVal>());
    if buffers.len() == 1 {
        // 如果在同一页
        let timeval: &mut TimeVal = unsafe { &mut *(buffers[0].as_mut_ptr() as *mut TimeVal) };
        timeval.sec = sec;
        timeval.usec = usec;
    } else if buffers.len() == 2 {
        // 跨页处理
        let timeval: &mut TimeVal = unsafe { &mut *(buffers[0].as_mut_ptr() as *mut TimeVal) };
        timeval.sec = sec;
        timeval.usec = usec;
    } else {
        // 错误处理
        return -1;
    }
    
    0
}


/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
// pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
//     trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
//     -1
// }

use core::mem;


pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    if ti.is_null() {
        return -1;
    }

    // 获取当前任务的用户空间 token
    let token = current_user_token();

    // 转换 `TaskInfo` 指针为内核可访问的缓冲区，通过分页处理可能的跨页情况
    let buffer = match translated_byte_buffer(token, ti as *const u8, mem::size_of::<TaskInfo>()) {
        Ok(buffer) => buffer,
        Err(_) => return -1,
    };

    // 将缓冲区转换为 `TaskInfo` 可变引用
    let ti_ref = unsafe { &mut *(buffer.as_mut_ptr() as *mut TaskInfo) };

    // 获取当前任务信息
    let inner = TASK_MANAGER.exclusive_access();
    let current_task_id = inner.current_task;
    let current_task = &inner.tasks[current_task_id];

    // 填充 `TaskInfo` 结构体的信息
    ti_ref.status = current_task.task_status;

    // 使用当前时间减去任务启动时间的假定方法来估算运行时间（此处用具体方法替代具体实现）
    ti_ref.time = calculate_runtime(current_task.); // 假设有一个方法能提供当前时间

    // 填充系统调用次数
    ti_ref.syscall_times = current_task.syscall_count.clone();

    0
}

// 假设的运行时间获取方法，用于替代具体实现
fn calculate_runtime(stime: usize) -> usize {
    let ctime = get_time_ms(); // 这里替代为具体获取时间的方法
    ctime - stime
}


// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
//     trace!("kernel: sys_mmap");

//     // 检查参数的合法性
//     if !is_page_aligned(start) || len == 0 || (port & 0x7) != 0 {
//         return -1;
//     }

//     let permissions = MapPermission::from_port(port);

//     // 申请内存区域
//     let result = allocate_virtual_memory(start, len, permissions);
//     if result.is_err() {
//         return -1;
//     }

//     start as isize
// }


// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}

// pub fn sys_munmap(start: usize, len: usize) -> isize {
//     trace!("kernel: sys_munmap");

//     // 检查参数的合法性
//     if !is_page_aligned(start) || len == 0 {
//         return -1;
//     }

//     let num_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE; // 计算需要取消映射的页数

//     // 获取当前的 MemorySet (页表集合)，并遍历要取消的页面范围
//     let memory_set = get_current_memory_set();
    
//     for i in 0..num_pages {
//         let vpn = VirtAddr::from(start + i * PAGE_SIZE).floor(); // 计算虚拟页号
//         if memory_set.translate(vpn).is_none() {
//             // 如果地址没有被映射，返回错误
//             return -1;
//         }
//         memory_set.page_table.unmap(vpn); // 取消映射
//     }

//     0 // 成功返回 0
// }

// // 帮助函数，检查地址是否对齐到页面大小
// fn is_page_aligned(addr: usize) -> bool {
//     addr % PAGE_SIZE == 0
// }

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
