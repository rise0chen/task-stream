#![no_std]

mod local;

use ach_mpmc::Mpmc;
pub use async_task::Runnable;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_util::task::AtomicWaker;
use futures_util::Stream;
pub use local::*;

const TASK_LEN: usize = 64;
static EXEC: Executor<TASK_LEN> = Executor::new();

pub struct Executor<const N: usize> {
    queue: Mpmc<Runnable, N>,
    waker: AtomicWaker,
}
impl<const N: usize> Executor<N> {
    pub const fn new() -> Self {
        Self {
            queue: Mpmc::new(),
            waker: AtomicWaker::new(),
        }
    }
    pub const fn into_local(self) -> LocalExecutor<N> {
        LocalExecutor::new(self)
    }

    /// task is waked
    fn schedule(&self, runnable: Runnable) {
        self.queue.push(runnable).unwrap();
        self.waker.wake();
    }

    pub fn spawn<F>(&'static self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (runnable, task) = async_task::spawn(future, move |r| self.schedule(r));
        runnable.schedule();
        task.detach();
    }

    /// # Safety
    ///
    /// - TaskStream must be used  on the original thread.
    unsafe fn spawn_local<F>(&self, future: F)
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let (runnable, task) = async_task::spawn_unchecked(future, move |r| self.schedule(r));
        runnable.schedule();
        task.detach();
    }

    pub fn stream(&self) -> TaskStream<N> {
        TaskStream { exec: self }
    }
}

pub fn spawn<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    EXEC.spawn(future)
}

pub fn stream() -> TaskStream<'static, TASK_LEN> {
    EXEC.stream()
}

pub struct TaskStream<'a, const N: usize> {
    exec: &'a Executor<N>,
}
impl<'a, const N: usize> TaskStream<'a, N> {
    pub fn get_task(&self) -> Option<Runnable> {
        self.exec.queue.pop().ok()
    }
}
impl<'a, const N: usize> Stream for TaskStream<'a, N> {
    type Item = Runnable;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(task) = self.get_task() {
            Poll::Ready(Some(task))
        } else {
            self.exec.waker.register(cx.waker());
            Poll::Pending
        }
    }
}
