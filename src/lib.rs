#![no_std]

use async_task::Runnable;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use futures_core::stream::Stream;
use futures_intrusive::timer::{Clock, MockClock};
use futures_intrusive::timer::{GenericTimerService, Timer, TimerFuture};
use heapless::mpmc::MpMcQueue;
use spin::{Lazy, Mutex};

const SPSC_LEN: usize = 32;
static CLOCK: MockClock = MockClock::new();
static TIMER: Lazy<GenericTimerService<Mutex<()>>> = Lazy::new(|| GenericTimerService::new(&CLOCK));
static EXEC: Executor = Executor::new();

pub struct Executor {
    queue: MpMcQueue<Runnable, SPSC_LEN>,
    waker: Mutex<Option<Waker>>,
}
impl Executor {
    const fn new() -> Self {
        Self {
            queue: MpMcQueue::new(),
            waker: Mutex::new(None),
        }
    }
}

fn schedule(runnable: Runnable) {
    EXEC.queue.enqueue(runnable).unwrap();
    if let Some(waker) = EXEC.waker.lock().take() {
        waker.wake();
    }
}
pub fn spawn<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (runnable, task) = async_task::spawn(future, schedule);
    runnable.schedule();
    task.detach();
}

pub fn tick(tick: u64) {
    CLOCK.set_time(CLOCK.now() + tick);
    TIMER.check_expirations();
}
pub fn now() -> u64 {
    CLOCK.now()
}
pub fn sleep(delay: Duration) -> TimerFuture<'static> {
    TIMER.delay(delay)
}

pub struct TaskStream {
    _inner: (),
}
impl TaskStream {
    pub fn stream() -> Self {
        TaskStream { _inner: () }
    }
    pub fn get_task(&self) -> Option<Runnable> {
        EXEC.queue.dequeue()
    }
}
impl Stream for TaskStream {
    type Item = Runnable;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(task) = self.get_task() {
            cx.waker().wake_by_ref();
            Poll::Ready(Some(task))
        } else {
            *EXEC.waker.lock() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
