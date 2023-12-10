use std::{sync::Arc, borrow::BorrowMut, task::{Context, Poll}, char::REPLACEMENT_CHARACTER, cell::RefCell, io};

use crossbeam::channel::{Receiver, Sender};
use polling::Events;

use crate::{job::Job, waker::waker_fn, reactor::Reactor};

pub struct Worker {
    pub id: usize,
    pub thread: Option<std::thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, stack_size: usize, receiver: Receiver<Arc<Job>>, sender: Sender<Arc<Job>>) -> Worker {
        println!("Worker {} is starting", id);
        let thread = std::thread::Builder::new()
            .name(format!("Worker {}", id))
            .stack_size(stack_size)
            .spawn(move || {
                thread_local! {
                    static REACTOR: RefCell<Reactor> = RefCell::new(Reactor::new());
                }
                let recv = receiver.clone();
                loop {
                    while let Ok(job) = recv.try_recv() {
                        let waker = {
                            let send = sender.clone();
                            let job = job.clone();
                            let waker = waker_fn(move || {
                                send.send(job.clone()).unwrap();
                            });
                            waker
                        };
                        match job.future.lock().as_mut().poll(&mut Context::from_waker(&waker)) {
                            Poll::Ready(_) => { },
                            Poll::Pending => {
                                sender.send(job.clone()).unwrap();
                                break;
                            }
                        }
                    }
                    if !REACTOR.with(|current| current.borrow().waiting_on_events()) {
                        println!("Worker {} is exiting", id);
                        break;
                    }
                    REACTOR.with(|current| {
                        let mut events = Events::new();
                        {
                            let mut reactor = current.borrow_mut();
                            reactor.wait(&mut events, None).unwrap();
                        }
                        let wakers = {
                            let mut reactor = current.borrow_mut();
                            reactor.drain(events)
                        };
                        for waker in wakers {
                            waker.wake();
                        }
                    });
                }
            })
            .unwrap();
        Worker {
            id,
            thread: Some(thread),
        }
    }

}