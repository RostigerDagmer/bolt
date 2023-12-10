use std::{task::{Waker, Context}, time::Duration, io, cell::RefCell};

use nohash_hasher::IntMap as Hashmap;
use polling::{Poller, Event, AsSource, AsRawSource, Events};

thread_local! {
    static REACTOR: RefCell<Reactor> = RefCell::new(Reactor::new());
}

pub struct Reactor {
    readable: Hashmap<u64, Vec<Waker>>, // fd -> wakers
    writable: Hashmap<u64, Vec<Waker>>,
    poller: Poller,
}

impl Reactor {
    pub fn new() -> Reactor {
        Reactor {
            readable: Hashmap::default(),
            writable: Hashmap::default(),
            poller: Poller::new().unwrap(),
        }
    }

    pub fn register<S: AsRawSource>(&mut self, fd: S) {
        let fd = fd.raw();
        unsafe {self.poller.add(fd, self.get_interest(fd)).unwrap()};
    }

    pub fn get_interest(&self, key: u64) -> Event {
        let readable = self.readable.contains_key(&key);
        let writable = self.writable.contains_key(&key);
        match (readable, writable) {
            (true, true) => Event::none(key as usize),
            (true, false) => Event::readable(key as usize),
            (false, true) => Event::writable(key as usize),
            (false, false) => Event::all(key as usize),
        }
    }

    pub fn deregister<S: AsSource + AsRawSource>(&mut self, fd: S) {
        let key = fd.raw();
        let fd = fd.as_socket();
        self.poller.delete(fd).unwrap();
        self.readable.remove(&key);
        self.writable.remove(&key);
    }

    // pub fn poll(&mut self, timeout: Option<Duration>) -> io::Result<()> {
    //     let events = self.poller.poll(timeout)?;
    //     for event in events {
    //         let fd = event.key;
    //         let wakers = match event.interest {
    //             Interest::READABLE => self.readable.get_mut(&fd),
    //             Interest::WRITABLE => self.writable.get_mut(&fd),
    //         };
    //         if let Some(wakers) = wakers {
    //             for waker in wakers.drain(..) {
    //                 waker.wake();
    //             }
    //         }
    //     }
    //     Ok(())
    // }
    pub fn drain(&mut self, events: Events) -> Vec<Waker> {
        let mut wakers = Vec::new();
        for ev in events.iter() {
            if let Some((_, readers)) = self.readable.remove_entry(&(ev.key as u64)) {
                wakers.extend(readers);
            }
            if let Some((_, writers)) = self.writable.remove_entry(&(ev.key as u64)) {
                wakers.extend(writers);
            }
        }
        wakers
    }

    pub fn wake_on_readable<S: AsSource + AsRawSource>(&mut self, fd: S, ctx: &mut Context<'_>) {
        let key = fd.raw();
        self.readable.entry(key).or_default().push(ctx.waker().clone());
        self.poller.modify(fd, self.get_interest(key)).unwrap();
    }
    
    pub fn wake_on_writable<S: AsSource + AsRawSource>(&mut self, fd: S, ctx: &mut Context<'_>) {
        let key = fd.raw();
        self.writable.entry(key).or_default().push(ctx.waker().clone());
        self.poller.modify(fd, self.get_interest(key)).unwrap();
    }

    pub fn wait(&mut self, events: &mut Events, timeout: Option<Duration>) -> io::Result<()> {
        self.poller.wait(events, timeout)?;
        Ok(())
    }

    pub fn waiting_on_events(&self) -> bool {
        !self.readable.is_empty() || !self.writable.is_empty()
    }

}