# Task-Stream

task-stream is a global task executor, can run in `no_std`.

## Usage

### spawn

```rust
async fn async_task() {
    println!("async_task.");
}
task_stream::spawn(async_task());
```

### executor

#### without async executor

```rust
use core::time::Duration;
use std::thread;

fn main() {
    let stream = task_stream::stream();
    loop {
        while let Some(task) = stream.get_task() {
            task.run();
        }
        thread::sleep(Duration::from_millis(100));
    }
}
```

#### use async executor

```rust
use async_std::prelude::*;
use async_std::task;

fn main() {
    task::spawn(async {
        let mut stream = task_stream::stream();
        while let Some(task) = stream.next().await {
            task.run();
        }
    });
}
```
