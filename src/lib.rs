#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll, Waker};
use fixed_queue::spsc::{Receiver, Sender, Spsc};
use futures_core::stream::Stream;
use spin::Mutex;

pub static TASKS: Tasks = Tasks::new();
const SPSC_LEN: usize = 10;

pub enum TaskType {
    // 立即执行
    Immediately,
    // n毫秒后执行一次
    Timeout(u64),
    // 每n毫秒执行一次
    Interval(u64),
}
enum TaskPoint {
    Sync(fn() -> ()),
    Async(Pin<Box<dyn Future<Output = ()> + Send + Sync>>),
}
struct Task {
    t: TaskType,
    f: TaskPoint,
    last: u64,
}

pub struct Tasks {
    tasks: Mutex<Vec<Task>>,
    spsc: Spsc<TaskPoint, SPSC_LEN>,
    waker: Mutex<Option<Waker>>,
}
impl Tasks {
    /// 设置当前时间，单位毫秒
    pub const fn new() -> Self {
        Tasks {
            tasks: Mutex::new(Vec::new()),
            spsc: Spsc::new(),
            waker: Mutex::new(None),
        }
    }
    pub(crate) fn set_waker(&self, waker: Waker) {
        *self.waker.lock() = Some(waker);
    }
    pub(crate) fn take_waker(&self) -> Option<Waker> {
        self.waker.lock().take()
    }
    /// 添加同步任务
    pub fn add_task(&self, t: TaskType, f: fn() -> ()) {
        let task = Task {
            t: t,
            f: TaskPoint::Sync(f),
            last: 0,
        };
        self.tasks.lock().push(task);
    }
    /// 添加异步任务
    pub fn add_async_task(&self, t: TaskType, f: Pin<Box<dyn Future<Output = ()> + Send + Sync>>) {
        if let TaskType::Interval(_) = t {
            panic!("task_stream interval task not support async task.");
        }
        let task = Task {
            t: t,
            f: TaskPoint::Async(f),
            last: 0,
        };
        self.tasks.lock().push(task);
    }
    /// 生成clock
    pub fn clock(&self) -> Clock {
        let sender = self.spsc.take_sender().unwrap();
        Clock {
            tasks: self,
            clock: AtomicU64::new(1),
            sender: sender,
        }
    }
    /// 生成TaskStream
    pub fn stream(&self) -> TaskStream {
        let recver = self.spsc.take_recver().unwrap();
        TaskStream {
            tasks: self,
            recver: recver,
        }
    }
}

pub struct Clock<'a> {
    tasks: &'a Tasks,
    clock: AtomicU64,
    sender: Sender<'a, TaskPoint, SPSC_LEN>,
}
impl<'a> Clock<'a> {
    /// 获取当前时间，单位毫秒
    pub fn now(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    /// tick毫秒执行一次
    pub fn tick(&self, tick: u64) {
        let now = self.clock.fetch_add(tick, Ordering::Relaxed);

        let mut tasks = self.tasks.tasks.lock();
        let tasks_len = tasks.len();
        for i in 0..tasks_len {
            let task = &mut tasks[tasks_len - 1 - i];
            if task.last == 0 {
                task.last = now;
                continue;
            }
            match task.t {
                TaskType::Immediately => {
                    let task = tasks.swap_remove(tasks_len - 1 - i);
                    self.ready(task.f)
                }
                TaskType::Timeout(t) => {
                    if now - task.last < t {
                        continue;
                    }
                    let task = tasks.swap_remove(tasks_len - 1 - i);
                    self.ready(task.f)
                }
                TaskType::Interval(t) => {
                    if now - task.last < t {
                        continue;
                    }
                    task.last = now;
                    if let TaskPoint::Sync(f) = task.f {
                        self.ready(TaskPoint::Sync(f))
                    } else {
                        panic!("task_stream interval task not support async task.");
                    }
                }
            }
        }
    }
    fn ready(&self, value: TaskPoint) {
        if let Err(_) = { self.sender.send(value) } {
            panic!("task too much.");
        };
        if let Some(warker) = self.tasks.take_waker() {
            warker.wake();
        }
    }
}

pub struct TaskStream<'a> {
    tasks: &'a Tasks,
    recver: Receiver<'a, TaskPoint, SPSC_LEN>,
}
impl<'a> Stream for TaskStream<'a> {
    type Item = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(task) = self.recver.try_recv() {
            let future = {
                match task {
                    TaskPoint::Async(f) => f,
                    TaskPoint::Sync(f) => Box::pin(async move { f() }),
                }
            };
            cx.waker().clone().wake();
            Poll::Ready(Some(future))
        } else {
            self.tasks.set_waker(cx.waker().clone());
            Poll::Pending
        }
    }
}
