//! # Tokio Async Tasks
//!
//! In this exercise, you will use `tokio::spawn` to create concurrent asynchronous tasks.
//!
//! ## Concepts
//! - `tokio::spawn` creates asynchronous tasks
//! - `JoinHandle` waits for task completion
//! - Concurrent execution between asynchronous tasks

use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// Concurrently compute the square of each number in 0..n, collect results and return in order.
///
/// Hint: Create `tokio::spawn` task for each i, collect JoinHandle, await them sequentially.
pub async fn concurrent_squares(n: usize) -> Vec<usize> {
    let mut handles = Vec::new();

    // 1. 批量创建任务 (Spawn)
    for i in 0..n {
        // spawn 会立即返回一个 JoinHandle，而任务已经开始在后台跑了
        let handle = spawn(async move {
            i * i
        });
        handles.push(handle);
    }

    // 2. 依次等待结果 (Await)
    let mut results = Vec::new();
    for handle in handles {
        // 这里的 await 是等待已经发出去的任务结束
        results.push(handle.await);
    }

    results
}

/// Concurrently execute multiple "time-consuming" tasks (simulated with sleep), return all results.
/// Each task sleeps `duration_ms` milliseconds and then returns its `task_id`.
///
/// Key: All tasks should execute concurrently, total duration should be close to single task duration, not sum of all tasks.
pub async fn parallel_sleep_tasks(n: usize, duration_ms: u64) -> Vec<usize> {
    let mut handles = Vec::new();

    // 1. 同时派发 n 个“睡觉”任务
    for i in 0..n {
        let handle = spawn(async move {
            // 模拟耗时操作
            sleep(duration_ms).await;
            i // 返回自己的 ID
        });
        handles.push(handle);
    }

    // 2. 收集所有任务的返回值
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }

    // 3. 排序（因为并发执行，返回顺序可能不固定）
    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_squares_basic() {
        let result = concurrent_squares(5).await;
        assert_eq!(result, vec![0, 1, 4, 9, 16]);
    }

    #[tokio::test]
    async fn test_squares_zero() {
        let result = concurrent_squares(0).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_squares_one() {
        let result = concurrent_squares(1).await;
        assert_eq!(result, vec![0]);
    }

    #[tokio::test]
    async fn test_parallel_sleep() {
        let start = Instant::now();
        let result = parallel_sleep_tasks(5, 100).await;
        let elapsed = start.elapsed();

        assert_eq!(result, vec![0, 1, 2, 3, 4]);
        // Concurrent execution, total time should be much less than 5 * 100ms
        assert!(
            elapsed.as_millis() < 400,
            "Tasks should run concurrently, took {}ms",
            elapsed.as_millis()
        );
    }
}
