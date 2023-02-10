use alloc::collections::VecDeque;

use super::Task;

pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> Self {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let dummy_waker = dummy_waker();
            let mut ctx = Context::from_waker(&dummy_waker);
            match task.poll(&mut ctx) {
                Poll::Ready(()) => {}
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }
}

use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
