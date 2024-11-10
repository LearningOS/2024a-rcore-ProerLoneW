//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::vec::Vec;
use alloc::vec;
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// get mutex stat
    fn stat(&self) -> isize;
    /// get allocation matrix
    fn get_allocation(&self) -> Option<Vec<usize>>;
    /// get Need matrix
    fn get_need(&self) -> Option<Vec<usize>>;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    allocate_tid: UPSafeCell<usize>,
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            allocate_tid: unsafe { UPSafeCell::new(0) },
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
    fn stat(&self) -> isize {
        let locked = self.locked.exclusive_access();
        if *locked == false {
            return 1;
        } else {
            return 0;
        }
    }

    fn get_allocation(&self) -> Option<Vec<usize>> {
        let locked = self.locked.exclusive_access();
        if *locked == false {
            return None;
        } else {
            return Some(vec![*self.allocate_tid.exclusive_access()]);
        }
    }

    fn get_need(&self) -> Option<Vec<usize>> {
        return None;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    allocate_tid: usize,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    allocate_tid: 0,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            let current_task = current_task().unwrap();
            let current_task_inner = current_task.inner_exclusive_access();
            mutex_inner.allocate_tid = current_task_inner.res.as_ref().unwrap().tid;
            drop(current_task_inner);
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
    fn stat(&self) -> isize {
        let mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            return 0;
        } else {
            return 1;
        }
    }

    fn get_allocation(&self) -> Option<Vec<usize>> {
        let mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            return Some(vec![mutex_inner.allocate_tid]);
        } else {
            return None;
        }
    }

    fn get_need(&self) -> Option<Vec<usize>> {
        let mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            let n = mutex_inner.wait_queue.len();
            if n == 0 {
                return None;
            } else {
                let mut res = Vec::new();
                for i in 0..n {
                    let task = &mutex_inner.wait_queue[i];
                    let current_task_inner = task.inner_exclusive_access();
                    res.push(current_task_inner.res.as_ref().unwrap().tid);
                    drop(current_task_inner);
                }
                return Some(res);
            }
        } else {
            return None;
        }
    }
}
