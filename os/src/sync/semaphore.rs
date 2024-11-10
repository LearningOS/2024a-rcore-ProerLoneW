//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub allocate_tid: Vec<usize>,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    allocate_tid: Vec::new(),
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        let current_task = current_task().unwrap();
        let current_task_inner = current_task.inner_exclusive_access();
        let tid = current_task_inner.res.as_ref().unwrap().tid;
        drop(current_task_inner);
        let n = inner.allocate_tid.len();
        for i in 0..n {
            if inner.allocate_tid[i] == tid {
                inner.allocate_tid.remove(i);
                break;
            }
        }
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let current_task_inner = current_task.inner_exclusive_access();
                let tid = current_task_inner.res.as_ref().unwrap().tid;
                inner.allocate_tid.push(tid);
                wakeup_task(task);
                drop(current_task_inner);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        } else {
            let current_task = current_task().unwrap();
            let current_task_inner = current_task.inner_exclusive_access();
            let tid = current_task_inner.res.as_ref().unwrap().tid;
            inner.allocate_tid.push(tid);
            drop(current_task_inner);
        }
    }
    ///stat
    pub fn stat(&self) -> isize {
        let inner = self.inner.exclusive_access();
        if inner.count < 0 {
            return 0;
        }
        return inner.count;
    }

    ///get allocation
    pub fn get_allocation(&self) -> Option<Vec<usize>> {
        let inner = self.inner.exclusive_access();
        if inner.allocate_tid.len() > 0 {
            return Some(inner.allocate_tid.clone());
        } else {
            return None;
        }
    }

    ///get need
    pub fn get_need(&self) -> Option<Vec<usize>> {
        let inner = self.inner.exclusive_access();
        let n = inner.wait_queue.len();
        if n == 0 {
            return None;
        } else {
            let mut res = Vec::new();
            for i in 0..n {
                let task = &inner.wait_queue[i];
                let current_task_inner = task.inner_exclusive_access();
                res.push(current_task_inner.res.as_ref().unwrap().tid);
                drop(current_task_inner);
            }
            return Some(res);
        }
    }
}
