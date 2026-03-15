//! # Mutex Shared State
//!
//! In this exercise, you will use `Arc<Mutex<T>>` to safely share and modify data between multiple threads.
//!
//! ## Concepts
//! - `Mutex<T>` mutex protects shared data
//! - `Arc<T>` atomic reference counting enables cross-thread sharing
//! - `lock()` acquires the lock and accesses data

use std::sync::{Arc, Mutex};
use std::thread;

/// Increment a counter concurrently using `n_threads` threads.
/// Each thread increments the counter `count_per_thread` times.
/// Returns the final counter value.
///
/// Hint: Use `Arc<Mutex<usize>>` as the shared counter.
pub fn concurrent_counter(n_threads: usize, count_per_thread: usize) -> usize {
    // 1. 创建一个被 Mutex 保护的初始值为 0 的计数器
    // 2. 使用 Arc 包裹它，以便在多个线程间安全地共享引用计数
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..n_threads {
        // 克隆 Arc 指针，增加引用计数，传入新线程
        let counter_clone = Arc::clone(&counter);
        
        let handle = thread::spawn(move || {
            for _ in 0..count_per_thread {
                // 获取锁。lock() 返回一个 Result，unwrap() 用于处理可能的锁中毒
                let mut num = counter_clone.lock().expect("Mutex poisoned");
                
                // 修改内部数据，num 会在作用域结束时自动释放锁（RAII）
                *num += 1;
            }
        });
        
        handles.push(handle);
    }

    // 等待所有线程执行完毕
    for handle in handles {
        handle.join().unwrap();
    }

    // 此时所有线程已结束，安全地取出最终值
    // 使用 * 获取 MutexGuard 内部的值，再通过 .unwrap() 之后的值其实在作用域内
    let final_result = *counter.lock().expect("Mutex poisoned");
    
    final_result
}

/// Add elements to a shared vector concurrently using multiple threads.
/// Each thread pushes its own id (0..n_threads) to the vector.
/// Returns the sorted vector.
///
/// Hint: Use `Arc<Mutex<Vec<usize>>>`.
pub fn concurrent_collect(n_threads: usize) -> Vec<usize> {
    let shared_vec = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for i in 0..n_threads {
        let vec_clone = Arc::clone(&shared_vec);
        
        let handle = thread::spawn(move || {
            let mut vec = vec_clone.lock().expect("Mutex poisoned");
            vec.push(i);
        });
        
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 获取锁并克隆数据以返回
    let mut result = shared_vec.lock().expect("Mutex poisoned").clone();
    
    // 对结果进行排序，确保输出顺序一致
    result.sort();
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_single_thread() {
        assert_eq!(concurrent_counter(1, 100), 100);
    }

    #[test]
    fn test_counter_multi_thread() {
        assert_eq!(concurrent_counter(10, 100), 1000);
    }

    #[test]
    fn test_counter_zero() {
        assert_eq!(concurrent_counter(5, 0), 0);
    }

    #[test]
    fn test_collect() {
        let result = concurrent_collect(5);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_collect_single() {
        assert_eq!(concurrent_collect(1), vec![0]);
    }
}
