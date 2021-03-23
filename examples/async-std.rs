use async_std::prelude::*;
use async_std::task;
use core::time::Duration;
use task_stream::{TaskType, TASKS};

fn sync_task() {
    println!("sync_task.");
}
async fn async_task() {
    println!("async_task.");
}

async fn async_main() {
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

    task::spawn(async {
        let mut stream = TASKS.stream();
        while let Some(fut) = stream.next().await {
            task::spawn(fut);
        }
    });
    let clock = TASKS.clock();
    loop {
        task::sleep(Duration::from_secs(10)).await;
        clock.tick(100);
    }
}
fn main() {
    task::block_on(async_main());
}
