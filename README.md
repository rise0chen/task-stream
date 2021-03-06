# Task-Stream

task-stream is a global task spawner, can run in `no_std`.

It provide spawner for async task, and asynchronous delay function.

It design for crate-creator. In third-party crate, can spawn a sub-task without care which executor main-program use.

## Usage

### third-party carte

```rust
fn test_sync_fun() {
    fn sync_task() {
        println!("sync_task.");
    }
    task_stream::spawn(async {
        sync_task();
    });
}
fn test_async_fun() {
    async fn async_task() {
        println!("async_task.");
    }
    task_stream::spawn(async_task());
}
fn test_capture_var() {
    let a: usize = 1;
    task_stream::spawn(async move {
        println!("catch a: {}.", a);
    });
}
fn test_sleep() {
    task_stream::spawn(async move {
        let mut now: u64 = 0;
        loop {
            println!("now: {}.", now);
            task_stream::sleep(Duration::from_millis(1000)).await;
            now += 1000;
        }
    });
}
```

### main-program

#### without async executor

```rust
use core::time::Duration;
use std::thread;
use task_stream::TaskStream;

fn sync_executor() {
    thread::spawn(|| {
        let stream = TaskStream::stream();
        loop {
            while let Some(task) = stream.get_task() {
                task.run();
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    loop {
        thread::sleep(Duration::from_millis(100));
        task_stream::tick(100);
    }
}
```

#### use async executor

```rust
use async_std::prelude::*;
use async_std::task;
use core::time::Duration;
use task_stream::TaskStream;

async fn async_executor() {
    task::spawn(async {
        let mut stream = TaskStream::stream();
        while let Some(task) = stream.next().await {
            task.run();
        }
    });
    loop {
        task::sleep(Duration::from_millis(100)).await;
        task_stream::tick(100);
    }
}
```
