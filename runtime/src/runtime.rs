use crate::threadpool::ThreadPool;

type Handle = crossbeam::channel::Sender<()>;

pub struct Scheduler {

}

struct Runtime {
    pub scheduler: Scheduler,
    pub thread_pool: ThreadPool,
    pub handle: Handle,
}