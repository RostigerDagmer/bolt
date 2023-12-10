use std::sync::Arc;

use crossbeam::channel::{self, Receiver, Sender};
use crossbeam::channel::bounded;
use crate::worker::Worker;
use crate::job::Job;

pub struct ThreadPool {
    pub workers: Vec<Worker>,
    pub sender: Sender<Arc<Job>>,
    pub receiver: Receiver<Arc<Job>>,
    pub config: ThreadPoolConfig,
}

pub struct ThreadPoolConfig {
    pub(crate) thread_count: usize,
    pub(crate) stack_size: Option<usize>,
    pub(crate) queue_size: Option<usize>,
}

impl ThreadPoolConfig {
    pub (crate) fn builder() -> ThreadPoolConfig {
        ThreadPoolConfig {
            thread_count: 1,
            stack_size: None,
            queue_size: None,
        }
    }
    pub(crate) fn thread_count(mut self, thread_count: usize) -> Self {
        self.thread_count = thread_count;
        self
    }
    pub(crate) fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }
    pub(crate) fn queue_size(mut self, queue_size: usize) -> Self {
        self.queue_size = Some(queue_size);
        self
    }
    pub(crate) fn build(self) -> ThreadPool {
        let stack_size = self.stack_size.unwrap_or(16 * 1024 * 1024);
        let queue_size = self.queue_size.unwrap_or(1024);
        let size = self.thread_count;
        assert!(size > 0);
        let (sender, receiver) = bounded::<Arc<Job>>(queue_size);
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, stack_size, receiver.clone(), sender.clone()));
        }
        ThreadPool {
            workers,
            sender,
            receiver,
            config: self,
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            worker.thread.take().unwrap().join().unwrap();
        }
    }
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        ThreadPoolConfig::builder().build()
    }
    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let job = Job::new(future);
        self.sender.send(Arc::new(job)).unwrap();
    }

    pub fn spawn_blocking<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (sender, receiver) = channel::bounded(1);
        self.sender.send(Arc::new(Job::new(async move {
            let _ = sender.send(f());
        }))).unwrap();
        receiver.recv().unwrap()
    }

    pub fn run(&mut self) {
        
    }

}
