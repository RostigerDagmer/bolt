pub mod threadpool;
pub mod runtime;
pub mod worker;
pub mod job;
pub mod waker;
pub mod reactor;

#[cfg(test)]
mod tests {
    use std::iter::Sum;

    use super::threadpool::ThreadPoolConfig;

    #[test]
    fn construct_pool() {
        let pool = ThreadPoolConfig::builder()
            .thread_count(4)
            .stack_size(16 * 1024 * 1024)
            .queue_size(1024)
            .build();

        assert_eq!(pool.workers.len(), 4);
    }

    #[test]
    fn execute_async() {
        let pool = ThreadPoolConfig::builder()
            .thread_count(4)
            .stack_size(16 * 1024 * 1024)
            .queue_size(1024)
            .build();

        let futures = (0..128).map(|i| async move { 
            let mut rand = 0;
            for _ in 0..10000 {
                rand = rand::random::<i32>();
            }
            println!("task: {} = {}", i, rand);
        });
        for future in futures {
            pool.spawn(future);
        }
        println!("done");
    }
}
