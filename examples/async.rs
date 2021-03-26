use async_std::prelude::*;
use async_std::task;
use core::time::Duration;
use task_stream::TaskStream;

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
        loop {
            println!("now: {}.", task_stream::now());
            task_stream::sleep(Duration::from_millis(1000)).await;
        }
    });
}

async fn async_executor() {
    task::spawn(async {
        let mut stream = TaskStream::stream();
        while let Some(task) = stream.next().await {
            task.run();
        }
    });
    loop {
        let now = chrono::Local::now().timestamp_millis();
        task::sleep(Duration::from_millis(100)).await;
        let tick = chrono::Local::now().timestamp_millis() - now;
        task_stream::tick(tick as u64);
    }
}
fn main() {
    test_sync_fun();
    test_async_fun();
    test_capture_var();
    test_sleep();

    task::block_on(async_executor());
}
