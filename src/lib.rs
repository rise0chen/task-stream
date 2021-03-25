#![no_std]

use async_task::Runnable;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use fixed_queue::spsc::{Receiver, Sender, Spsc};
use futures_core::stream::Stream;
use futures_intrusive::timer::{Clock, MockClock};
use futures_intrusive::timer::{GenericTimerService, Timer, TimerFuture};
use spin::{Lazy, Mutex};

const SPSC_LEN: usize = 30;
static SPSC: Spsc<Runnable, SPSC_LEN> = Spsc::new();
static SENDER: Lazy<Sender<'static, Runnable, SPSC_LEN>> =
    Lazy::new(|| SPSC.take_sender().unwrap());
static RECVER: Lazy<Receiver<'static, Runnable, SPSC_LEN>> =
    Lazy::new(|| SPSC.take_recver().unwrap());
static WAKER: Mutex<Option<Waker>> = Mutex::new(None);
static CLOCK: MockClock = MockClock::new();
static TIMER: Lazy<GenericTimerService<Mutex<()>>> = Lazy::new(|| GenericTimerService::new(&CLOCK));

fn schedule(runnable: Runnable) {
    SENDER.send(runnable).unwrap();
    if let Some(waker) = WAKER.lock().take() {
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
        if let Ok(task) = RECVER.try_recv() {
            Some(task)
        } else {
            None
        }
    }
}
impl Stream for TaskStream {
    type Item = Runnable;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(task) = RECVER.try_recv() {
            cx.waker().wake_by_ref();
            Poll::Ready(Some(task))
        } else {
            *WAKER.lock() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
