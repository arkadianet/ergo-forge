//! Running engine work on the blocking pool under ONE budget that is held
//! for the whole job, not just for the request.
//!
//! A `tower` concurrency layer releases its permit when the handler future
//! is dropped — which is what happens when a client disconnects — while the
//! `spawn_blocking` job it started keeps running. Repeated aborted requests
//! could therefore exceed the intended bound on large-stack threads. Here
//! the permit is an `OwnedSemaphorePermit` moved INTO the blocking closure,
//! so it is released only when the job itself finishes.

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// The engine budget: at most `n` jobs on the blocking pool at once.
#[derive(Clone)]
pub struct EngineBudget {
    sem: Arc<Semaphore>,
}

impl EngineBudget {
    pub fn new(n: usize) -> Self {
        EngineBudget {
            sem: Arc::new(Semaphore::new(n)),
        }
    }

    /// Permits not currently held by a running job.
    pub fn available(&self) -> usize {
        self.sem.available_permits()
    }

    /// Wait for a permit, then run `f` on the blocking pool inside the
    /// large-stack wrapper. The permit is dropped when `f` returns, even if
    /// the caller's future was dropped meanwhile. `None` only when the pool
    /// itself failed (a panic inside `f` is `None` too; handlers map it to
    /// an internal error).
    pub async fn run<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let permit: OwnedSemaphorePermit = self.sem.clone().acquire_owned().await.ok()?;
        tokio::task::spawn_blocking(move || {
            let _held = permit;
            ergo_sandbox::decompile::with_large_stack(f)
        })
        .await
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// The permit outlives a dropped request: with a budget of 1, a job
    /// whose caller aborted still holds the budget until it finishes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_permit_is_held_until_the_blocking_job_finishes() {
        let budget = EngineBudget::new(1);
        let b = budget.clone();
        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let s = started.clone();
        let handle = tokio::spawn(async move {
            b.run(move || {
                s.store(true, std::sync::atomic::Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(300));
                42
            })
            .await
        });
        while !started.load(std::sync::atomic::Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        handle.abort(); // the "client disconnected"
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            budget.available(),
            0,
            "permit released while the job still runs"
        );
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert_eq!(
            budget.available(),
            1,
            "permit not released after the job finished"
        );
    }
}
