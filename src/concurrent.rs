use ethers::providers::Middleware;
use futures::future;
use indicatif::ProgressBar;
use std::future::Future;
use std::sync::{Arc, Mutex};

pub async fn run_concurrent<'a, F, Fut, V, M>(
    start: u64,
    end: u64,
    step: usize,
    middleware: Arc<M>,
    func: F,
) -> Vec<V>
where
    F: Fn(u64, u64, Arc<M>, Option<Arc<Mutex<ProgressBar>>>) -> Fut,
    Fut: Future<Output = Vec<V>> + Send + 'a,
    V: Send + 'a,
    M: Middleware + 'a,
{
    let size = end - start;
    let pb = ProgressBar::new(size);
    let shared_pb = Arc::new(Mutex::new(pb));
    let mut futures: Vec<_> = vec![];

    for i in (start..end).step_by(step) {
        futures.push(func(
            i,
            (i + step as u64).min(end),
            middleware.clone(),
            Some(shared_pb.clone()),
        ));
    }
    future::join_all(futures)
        .await
        .into_iter()
        .flatten()
        .collect()
}
