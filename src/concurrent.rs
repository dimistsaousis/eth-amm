use ethers::providers::Middleware;
use futures::future;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct BatchError {
    pub start: usize,
    pub end: usize,
}

impl BatchError {
    pub fn new(start: usize, end: usize) -> Self {
        BatchError { start, end }
    }
}

pub async fn run_concurrent<'a, F, Fut, V, M>(
    start: usize,
    end: usize,
    step: usize,
    middleware: Arc<M>,
    func: F,
) -> Vec<V>
where
    F: Fn(usize, usize, Arc<M>, Option<Arc<Mutex<ProgressBar>>>) -> Fut,
    Fut: Future<Output = Result<Vec<V>, BatchError>> + Send + 'a,
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
    let results = future::join_all(futures).await;
    let mut combined_results = vec![];

    for result in results {
        match result {
            Ok(data) => combined_results.extend(data),
            Err(err) => {
                println!(
                    "Failed to get results from {} to end {} trying with step 1.",
                    err.start, err.end
                );
                let futures = (err.start..err.end)
                    .into_iter()
                    .map(|idx| func(idx, idx, middleware.clone(), Some(shared_pb.clone())));
                let results = future::join_all(futures).await;
                for result in results {
                    if let Ok(result) = result {
                        combined_results.extend(result);
                    }
                }
            }
        }
    }
    combined_results
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
    Fut: Future<Output = Result<HashMap<K, V>, BatchError>> + Send + 'a,
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

    let results = future::join_all(futures).await;
    let mut combined_results = HashMap::new();

    for result in results {
        match result {
            Ok(data) => data.into_iter().for_each(|(k, v)| {
                combined_results.insert(k, v);
            }),
            Err(err) => {
                println!(
                    "Failed to get results from {} to end {} trying with step 1.",
                    err.start, err.end
                );
                let futures = (err.start..err.end)
                    .into_iter()
                    .map(|idx| func(idx, idx, middleware.clone(), Some(shared_pb.clone())));
                let results = future::join_all(futures).await;
                for res in results {
                    if let Ok(res) = res {
                        if let Some((k, v)) = res.into_iter().next() {
                            combined_results.insert(k, v);
                        }
                    }
                }
            }
        }
    }
    combined_results
}
