use crate::{Executor, TaskStream};
use async_task::Runnable;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_util::Stream;

pub struct LocalRunnable {
    runnable: Runnable,
    // Prevent Send, Sync
    _private: PhantomData<*mut ()>,
}
impl LocalRunnable {
    fn new(runnable: Runnable) -> Self {
        Self {
            runnable,
            _private: PhantomData,
        }
    }
    pub fn run(self) -> bool {
        self.runnable.run()
    }
}

pub struct LocalExecutor<const N: usize> {
    exec: Executor<N>,
    // Prevent Send, Sync
    _private: PhantomData<*mut ()>,
}
impl<const N: usize> LocalExecutor<N> {
    pub const fn new(exec: Executor<N>) -> Self {
        Self {
            exec,
            _private: PhantomData,
        }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        unsafe { self.exec.spawn_local(future) }
    }

    pub fn stream(&self) -> LocalTaskStream<N> {
        LocalTaskStream {
            stream: self.exec.stream(),
            _private: PhantomData,
        }
    }
}

pub struct LocalTaskStream<'a, const N: usize> {
    stream: TaskStream<'a, N>,
    // Prevent Send, Sync
    _private: PhantomData<*mut ()>,
}
impl<'a, const N: usize> LocalTaskStream<'a, N> {
    pub fn get_task(&self) -> Option<LocalRunnable> {
        let runnable = self.stream.get_task()?;
        Some(LocalRunnable::new(runnable))
    }
}
impl<'a, const N: usize> Stream for LocalTaskStream<'a, N> {
    type Item = LocalRunnable;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(runnable) = Pin::new(&mut self.stream).poll_next(cx){
            if let Some(runnable) = runnable {
                Poll::Ready(Some(LocalRunnable::new(runnable)))
            }else{
                Poll::Ready(None)
            }
        }else{
            Poll::Pending
        }
    }
}
