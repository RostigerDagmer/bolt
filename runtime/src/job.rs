use std::{future::Future, pin::Pin};
use parking_lot::Mutex;

pub type LocalBoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;
pub struct Job {
    pub future: Mutex<LocalBoxedFuture<'static, ()>>,
}

impl Job {
    pub fn new<F>(future: F) -> Job
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Job {
            future: Mutex::new(Box::pin(future)),
        }
    }
}