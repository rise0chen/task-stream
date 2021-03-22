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
use spin::{Lazy, Mutex};

pub static TASKS: Lazy<Tasks> = Lazy::new(|| Tasks::new(0));
const SPSC_LEN: usize = 10;
static SPSC: Spsc<TaskPoint, SPSC_LEN> = Spsc::new();

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
    clock: AtomicU64,
    tasks: Mutex<Vec<Task>>,
    sender: Sender<'static, TaskPoint, 10>,
    waker: Mutex<Option<Waker>>,
}
impl Tasks {
    /// 设置当前时间，单位毫秒
    fn new(time: u64) -> Self {
        let sender = SPSC.take_sender().unwrap();
        Tasks {
            clock: AtomicU64::new(time),
            tasks: Mutex::new(Vec::new()),
            sender: sender,
            waker: Mutex::new(None),
        }
    }
    pub(crate) fn set_waker(&self, waker: Waker) {
        *self.waker.lock() = Some(waker);
    }
    /// 获取当前时间，单位毫秒
    fn now(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    /// 添加同步任务
    pub fn add_task(&self, t: TaskType, f: fn() -> ()) {
        let task = Task {
            t: t,
            f: TaskPoint::Sync(f),
            last: self.now(),
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
            last: self.now(),
        };
        self.tasks.lock().push(task);
    }
    /// tick毫秒执行一次
    pub fn tick(&self, tick: u64) {
        let now = self.clock.fetch_add(tick, Ordering::Relaxed);

        let mut tasks = self.tasks.lock();
        let tasks_len = tasks.len();
        for i in 0..tasks_len {
            let task = &mut tasks[tasks_len - 1 - i];
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
        if let Err(_) = self.sender.send(value) {
            panic!("task too much.");
        };
        if let Some(warker) = self.waker.lock().take() {
            warker.wake();
        }
    }
    /// 生成TaskStream
    pub fn stream(&self) -> TaskStream {
        let recver = SPSC.take_recver().unwrap();
        TaskStream {
            tasks: self,
            recver: recver,
        }
    }
}

pub struct TaskStream<'a> {
    tasks: &'a Tasks,
    recver: Receiver<'a, TaskPoint, 10>,
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
