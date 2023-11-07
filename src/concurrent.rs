use ethers::providers::Middleware;
use futures::future;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

pub async fn run_concurrent<'a, F, Fut, V, M>(
    start: usize,
    end: usize,
    step: usize,
    middleware: Arc<M>,
    func: F,
) -> Vec<V>
where
    F: Fn(usize, usize, Arc<M>, Option<Arc<Mutex<ProgressBar>>>) -> Fut,
    Fut: Future<Output = Vec<V>> + Send + 'a,
    V: Send + 'a,
    M: Middleware + 'a,
{
    let size = end - start;
    let pb = ProgressBar::new(size as u64);
    let shared_pb = Arc::new(Mutex::new(pb));
    let mut futures: Vec<_> = vec![];

    for i in (start..end).step_by(step) {
        futures.push(func(
            i,
            (i + step).min(end),
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

pub async fn run_concurrent_hash<'a, F, Fut, K, V, M>(
    start: usize,
    end: usize,
    step: usize,
    middleware: Arc<M>,
    func: F,
) -> HashMap<K, V>
where
    F: Fn(usize, usize, Arc<M>, Option<Arc<Mutex<ProgressBar>>>) -> Fut,
    Fut: Future<Output = HashMap<K, V>> + Send + 'a,
    K: Eq + Hash + Send + 'a,
    V: Send + 'a,
    M: Middleware + 'a,
{
    let size = end - start;
    let pb = ProgressBar::new(size as u64);
    let shared_pb = Arc::new(Mutex::new(pb));
    let mut futures: Vec<_> = vec![];

    for i in (start..end).step_by(step) {
        futures.push(func(
            i,
            (i + step).min(end),
            middleware.clone(),
            Some(shared_pb.clone()),
        ));
    }

    let mut result = HashMap::new(); // 'mut' keyword added
    future::join_all(futures).await.into_iter().for_each(|map| {
        map.into_iter().for_each(|(k, v)| {
            result.insert(k, v);
        });
    });
    result
}
