//! File and filesystem-related syscalls
use crate::fs::inode::ROOT_INODE;
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
// pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }

pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file_stat = file.stat();
        let user_stat = translated_refmut(token, st);
        *user_stat = file_stat;
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement linkat.
// pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }

// sys linkat
pub fn sys_linkat(oldpath: *const u8, newpath: *const u8) -> isize {
    let token = current_user_token();

    // 读取 oldpath 和 newpath 的字符串
    let oldpath = translated_str(token, oldpath);
    let newpath = translated_str(token, newpath);

    // 查找 oldpath 对应的 inode
    let target_inode = match open_file(&oldpath, OpenFlags::RDONLY) {
        Some(inode) => inode.inner_inode(),
        None => {
            println!("linkat failed: source file not found");
            return -1;
        }
    };

    // 增加引用计数
    target_inode.increment_nlink();

    // 尝试创建硬链接
    if ROOT_INODE.link(&newpath, target_inode.clone()) {
        0 // 成功返回 0
    } else {
        // link 失败，回退引用计数
        target_inode.decrement_nlink();
        -1 // 失败返回 -1
    }
}

/// YOUR JOB: Implement unlinkat.
// pub fn sys_unlinkat(_name: *const u8) -> isize {
//     trace!(
//         "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
//         current_task().unwrap().pid.0
//     );
//     -1
// }


pub fn sys_unlinkat(name: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, name);

    let inode = match open_file(&path, OpenFlags::RDONLY) {
        Some(file) => file.inner_inode(),
        None => {
            println!("unlinkat failed: file not found");
            return -1;
        }
    };

    // 标志是否需要清理 inode 数据
    let mut needs_clear = false;

    // 解除文件链接并更新引用计数
    let result = if ROOT_INODE.unlink(&path) {
        inode.decrement_nlink();

        // 如果引用计数为 0，则标记需要清理
        if inode.get_nlink() == 0 {
            needs_clear = true;
        }

        0 // 返回 0 表示成功
    } else {
        println!("unlinkat failed: unable to delete file");
        -1 // 返回 -1 表示失败
    };

    // 在退出锁的上下文后再执行清理操作
    if needs_clear {
        inode.clear();
    }

    result
}
