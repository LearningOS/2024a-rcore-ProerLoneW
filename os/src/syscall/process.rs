//! Process management syscalls

use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_byte_buffer, translated_refmut, translated_str, MapPermission, VirtAddr},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
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
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
// pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    // 获取时间（假设用 get_time_sec_usec 函数来获取秒和微秒）
    let sec = get_time_ms() / 1000;
    let usec = get_time_us();
    let token = current_user_token();

    // 使用 translated_byte_buffer 处理跨页情况
    let mut buffers =
        translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
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
//     trace!(
//         "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }
use core::mem;

pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if ti.is_null() {
        return -1;
    }

    // 获取当前任务的用户空间 token
    let token = current_user_token();

    // 通过分页转换，将 `TaskInfo` 指针转换为内核可访问的缓冲区
    let mut buffers =
        translated_byte_buffer(token, ti as *const u8, core::mem::size_of::<TaskInfo>());

    // 获取当前任务的信息
    let current_task = current_task().unwrap();
    let task_inner = current_task.inner_exclusive_access();

    // 处理分页缓冲区，填充 `TaskInfo` 的信息
    if buffers.len() == 1 {
        // 单页情况，直接填充
        let ti_ref = unsafe { &mut *(buffers[0].as_mut_ptr() as *mut TaskInfo) };
        ti_ref.status = task_inner.task_status;
        ti_ref.time = calculate_runtime(task_inner.start_time); // 假设有一个 `calculate_runtime` 函数获取当前运行时间
        ti_ref.syscall_times = task_inner.syscall_times.clone();
    } else if buffers.len() == 2 {
        // 跨页情况，分块填充
        let temp_info = TaskInfo {
            status: task_inner.task_status,
            time: calculate_runtime(task_inner.start_time), // 假设有一个 `calculate_runtime` 函数
            syscall_times: task_inner.syscall_times.clone(),
        };

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

fn calculate_runtime(_start_time: usize) -> usize {
    let current_time = get_time_ms(); // 这里替代为具体获取时间的方法
    current_time - _start_time
}

/// YOUR JOB: Implement mmap.
// pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }
use crate::config::PAGE_SIZE;
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap called",
        current_task().unwrap().pid.0
    );

    if current_task().is_none() {
        println!("mmap failed: unable to retrieve current task");
        return -1;
    }

    // 检查 start 是否对齐到页面大小
    if _start % PAGE_SIZE != 0 {
        println!(
            "mmap failed: start address is not page-aligned (start: {:#x}, len: {})",
            _start, _len
        );
        return -1;
    }

    // 检查 port 是否有效
    if _port & !0x7 != 0 || _port & 0x7 == 0 {
        println!("mmap failed: invalid or meaningless port (port: {:#x})", _port);
        return -1;
    }

    // 解析权限位
    let permission = match _port & 0x7 {
        0b001 => MapPermission::R,
        0b010 => MapPermission::W,
        0b011 => MapPermission::R | MapPermission::W,
        0b100 => MapPermission::X,
        0b101 => MapPermission::R | MapPermission::X,
        0b110 => MapPermission::W | MapPermission::X,
        0b111 => MapPermission::R | MapPermission::W | MapPermission::X,
        _ => return -1,
    };

    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);

    // 获取当前任务并访问其内存集
    let current_task = current_task().unwrap();
    let mut task_inner = current_task.inner_exclusive_access();
    let memory_set = &mut task_inner.memory_set;

    // 检查是否已映射
    if memory_set.check_vpn_range(start_va.floor(), end_va.floor()) {
        println!(
            "mmap failed: address range already mapped from {:#x} to {:#x}",
            _start,
            _start + _len
        );
        return -1;
    }

    // 插入映射区域
    memory_set.insert_framed_area(start_va, end_va, permission | MapPermission::U);
    println!(
        "mmap succeeded: start = {:#x}, len = {}, permission = {:?}",
        _start, _len, permission
    );

    0 // 成功返回 0
}


/// YOUR JOB: Implement munmap.
// pub fn sys_munmap(_start: usize, _len: usize) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap called",
        current_task().unwrap().pid.0
    );

    // 检查输入参数是否有效
    if _len == 0 || _start % PAGE_SIZE != 0 {
        println!(
            "munmap failed: invalid start address or length (start: {:#x}, len: {})",
            _start, _len
        );
        return -1;
    }

    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);

    // 获取当前任务
    if current_task().is_none() {
        println!("munmap failed: current task not found");
        return -1;
    }
    let current_task = current_task().unwrap();
    let mut task_inner = current_task.inner_exclusive_access();
    let memory_set = &mut task_inner.memory_set;

    // 使用封装的函数来查找完全匹配的区域
    if let Some(_area) = memory_set.find_exact_match(start_va, end_va) {
        memory_set.remove_area(start_va, end_va);
        println!(
            "munmap succeeded: unmapped range (start: {:#x}, end: {:#x})",
            _start,
            _start + _len
        );
        0
    } else {
        println!(
            "munmap failed: address range not fully mapped (start: {:#x}, end: {:#x})",
            _start,
            _start + _len
        );
        -1
    }
}


/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// Creates a new process and executes the specified ELF file.
/// Returns the new process's PID if successful, otherwise -1.
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn called",
        current_task().unwrap().pid.0
    );

    // 检查 `_path` 是否为 null 指针
    if _path.is_null() {
        println!("spawn failed: _path is a null pointer");
        return -1;
    }

    // 读取路径字符串
    let token = current_user_token();
    let path = translated_str(token, _path);

    // 查找目标 ELF 数据
    let elf_data = match get_app_data_by_name(path.as_str()) {
        Some(data) => data,
        None => {
            println!("spawn failed: app {} not found", path.as_str());
            return -1; // 如果找不到应用程序，返回 -1
        }
    };

    // 获取当前进程（父进程）的 `TaskControlBlock`
    let parent_task = current_task().unwrap();

    // 创建新的子进程
    let new_task = parent_task.fork();

    // 设置子进程的 ELF 文件并执行
    new_task.exec(elf_data);

    // 将子进程加入调度队列
    add_task(new_task.clone());

    // 返回子进程的 PID
    new_task.getpid() as isize
}

use crate::config::BIG_STRIDE;

// YOUR JOB: Set task priority.
// pub fn sys_set_priority(_prio: isize) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }

/// 设置当前任务的优先级
pub fn sys_set_priority(prio: isize) -> isize {
    if prio < 2 {
        println!("Error: Priority must be at least 2.");
        return -1; // 返回错误代码
    }

    let current_task = match current_task() {
        Some(task) => task,
        None => {
            println!("Error: No current task found.");
            return -1;
        }
    };

    // 获取任务的独占访问权限
    let mut task_inner = current_task.inner_exclusive_access();

    // 更新优先级
    task_inner.priority = prio as usize;

    // 计算并更新 stride 值
    task_inner.stride = BIG_STRIDE / task_inner.priority;

    // 如果是新创建的任务，pass 值初始化为 0，否则保持当前 pass 值
    if task_inner.pass == 0 {
        task_inner.pass = task_inner.stride; // 初始情况下 pass 等于 stride
    }

    println!(
        "Priority set to {}, stride calculated as {}, pass set to {}",
        task_inner.priority, task_inner.stride, task_inner.pass
    );

    task_inner.priority as isize // 成功返回 0
}
