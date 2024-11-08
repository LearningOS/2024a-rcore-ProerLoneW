//! Process management syscalls
// use alloc::borrow::ToOwned;

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
/// #[derive(Copy, Clone)]
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

use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;
use crate::task::TASK_MANAGER;
/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
// pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
//     trace!("kernel: sys_get_time");
//     -1
// }
use crate::timer::{get_time_ms, get_time_us};
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    // 获取时间（假设用 get_time_sec_usec 函数来获取秒和微秒）
    let sec: usize = get_time_ms() / 1000;
    let usec = get_time_us();
    // 使用 translated_byte_buffer 处理跨页情况
    let mut buffers = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
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

    // 通过分页转换，将 `TaskInfo` 指针转换为内核可访问的缓冲区
    let mut buffers = translated_byte_buffer(token, ti as *const u8, mem::size_of::<TaskInfo>());

    // 检查是否成功获取缓冲区
    if buffers.is_empty() {
        return -1;
    }

    // 获取当前任务的信息
    let inner = TASK_MANAGER.inner_exclusive_access();
    let current_task_id = inner.current_task;
    let current_task = &inner.tasks[current_task_id];

    // 处理分页缓冲区，填充 `TaskInfo` 的信息
    if buffers.len() == 1 {
        // 同一页，直接填充
        let ti_ref = unsafe { &mut *(buffers[0].as_mut_ptr() as *mut TaskInfo) };
        ti_ref.status = current_task.task_status;
        ti_ref.time = calculate_runtime(current_task.start_time); // 假设有一个 `calculate_runtime` 函数获取当前运行时间
        ti_ref.syscall_times = current_task.syscall_times;
    } else if buffers.len() == 2 {
        // 跨页情况，分块填充
        let mut temp_info = TaskInfo {
            status: current_task.task_status,
            time: calculate_runtime(current_task.start_time), // 假设有一个 `calculate_runtime` 函数
            syscall_times: [0; 500],
        };

        // 使用 `copy_from_slice` 将 `current_task.syscall_times` 的内容复制到 `temp_info.syscall_times` 中
        temp_info
            .syscall_times
            .copy_from_slice(&current_task.syscall_times);

        // 将 `TaskInfo` 的字节数据分两页写入
        let temp_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                &temp_info as *const TaskInfo as *const u8,
                mem::size_of::<TaskInfo>(),
            )
        };
        let (first_part, second_part) = temp_bytes.split_at(buffers[0].len());
        buffers[0].copy_from_slice(first_part);
        buffers[1].copy_from_slice(second_part);
    } else {
        // 错误情况
        return -1;
    }

    0
}

// get running time
fn calculate_runtime(stime: usize) -> usize {
    let ctime = get_time_ms(); // 这里替代为具体获取时间的方法
    ctime - stime
}

// YOUR JOB: Implement mmap.
// pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
//     trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
//     -1
// }

use crate::config::PAGE_SIZE;
use crate::mm::MapPermission;
use crate::mm::VirtAddr;
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // 允许长度为非页面大小整数倍，允许 len = 0
    if len == 0 {
        // println!("mmap failed: invalid length {}", len);
        return -1;
    }

    if start % PAGE_SIZE != 0 {
        // println!(
        //     "mmap failed: start address is not page-aligned (start: {:#x}, len: {})",
        //     start, len
        // );
        return -1;
    }

     // 检查 port 其他位是否为 0，且最低 3 位不能为 0
     if port & !0x7 != 0 {
        // println!("mmap failed: invalid port with non-zero extra bits (port: {:#x})", port);
        return -1;
    }
    if port & 0x7 == 0 {
        // println!("mmap failed: meaningless port with no permissions set (port: {:#x})", port);
        return -1;
    }

    // 解析权限位
    let permission = match port & 0x7 {
        0b001 => MapPermission::R,
        0b010 => MapPermission::W,
        0b011 => MapPermission::R | MapPermission::W,
        0b100 => MapPermission::X,
        0b101 => MapPermission::R | MapPermission::X,
        0b110 => MapPermission::W | MapPermission::X,
        0b111 => MapPermission::R | MapPermission::W | MapPermission::X,
        _ => {
            // println!("mmap failed: invalid port {}", port);
            return -1; // 无效的权限
        }
    };

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    // 获取当前任务并检查地址范围
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current_task_id = inner.current_task;
    let memory_set = &mut inner.tasks[current_task_id].memory_set;

    // 检查是否已经被映射
    if memory_set.check_vpn_range(start_va.floor(), end_va.floor()) {
        // println!(
        //     "mmap failed: address range already mapped from {:#x} to {:#x}",
        //     start,
        //     start + len
        // );
        return -1;
    }

    // 插入映射区域
    memory_set.insert_framed_area(start_va, end_va, permission | MapPermission::U);
    // println!(
    //     "mmap succeeded: start = {:#x}, len = {}, permission = {:?}",
    //     start, len, permission
    // );

    0 // 成功返回 0
}

// YOUR JOB: Implement munmap.
// pub fn sys_munmap(_start: usize, _len: usize) -> isize {
//     trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
//     -1
// }

pub fn sys_munmap(start: usize, len: usize) -> isize {
    if len == 0 || start % PAGE_SIZE != 0 || len % PAGE_SIZE != 0 {
        // println!("munmap failed: invalid start address or length (start: {:#x}, len: {})", start, len);
        return -1;
    }

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current_task_id = inner.current_task;
    let memory_set = &mut inner.tasks[current_task_id].memory_set;

    // 使用封装的函数来查找完全匹配的区域
    if let Some(_area) = memory_set.find_exact_match(start_va, end_va) {
        memory_set.remove_area(start_va, end_va);
        // println!("munmap succeeded: unmapped range (start: {:#x}, end: {:#x})", start, start + len);
        0
    } else {
        // println!("munmap failed: address range not fully mapped (start: {:#x}, end: {:#x})", start, start + len);
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
