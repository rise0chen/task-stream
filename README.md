# Task-Stream

task-stream is a global task spawner, can run in `no_std`.

It support the timer tasks, sync/async task.

It design for crate-creator. In third-party crate, can spawn a sub-task without care which executor user's app use.

## Usage

### third-party carte

```rust
use task_stream::{TaskType, TASKS};

fn sync_task() {
    println!("sync_task.");
}
async fn async_task() {
    println!("async_task.");
}
TASKS.add_task(TaskType::Interval(1000), sync_task);
TASKS.add_task(TaskType::Timeout(1000), || {
    println!("hello world.");
});
TASKS.add_async_task(TaskType::Timeout(1000), Box::pin(async_task()));
let a = 1;
TASKS.add_async_task(
    TaskType::Timeout(1000),
    Box::pin(async move {
        println!("hello async, {}.", a);
    }),
);
```

### user'a app

```rust
use async_std::prelude::*;
use async_std::task;
use core::time::Duration;
use task_stream::{TaskPoint, TASKS};

async fn async_main() {
    task::spawn(async {
        let mut stream = TASKS.stream();
        while let Some(task) = stream.next().await {
            match task {
                TaskPoint::Sync(f) => {
                    std::thread::spawn(f);
                }
                TaskPoint::Async(fut) => {
                    task::spawn(fut);
                }
            }
        }
    });
    let clock = TASKS.clock();
    loop {
        task::sleep(Duration::from_millis(100)).await;
        clock.tick(100);
    }
}
```
