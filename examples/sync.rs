use async_tick as tick;
use core::time::Duration;
use std::thread;

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
            println!("now: {:?}.", Duration::from_nanos(tick::now()));
            tick::sleep(Duration::from_millis(1000)).await;
        }
    });
}

fn main() {
    thread::spawn(|| {
        let mut ticker = tick::take_tick().unwrap();
        let period = Duration::from_millis(100);
        loop {
            thread::sleep(period);
            ticker.tick(period);
        }
    });

    test_sync_fun();
    test_async_fun();
    test_capture_var();
    test_sleep();

    let stream = task_stream::stream();
    loop {
        while let Some(task) = stream.get_task() {
            task.run();
        }
        thread::sleep(Duration::from_millis(100));
    }
}
