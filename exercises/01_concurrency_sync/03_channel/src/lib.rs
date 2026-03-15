//! # Channel Communication
//!
//! In this exercise, you will use `std::sync::mpsc` channels to pass messages between threads.
//!
//! ## Concepts
//! - `mpsc::channel()` creates a multiple producer, single consumer channel
//! - `Sender::send()` sends a message
//! - `Receiver::recv()` receives a message
//! - Multiple producers can be created via `Sender::clone()`

use std::sync::mpsc;
use std::thread;

/// Create a producer thread that sends each element from items into the channel.
/// The main thread receives all messages and returns them.
pub fn simple_send_recv(items: Vec<String>) -> Vec<String> {
    // 1. 创建频道：tx 是发送端 (Sender)，rx 是接收端 (Receiver)
    let (tx, rx) = mpsc::channel();

    // 2. 创建生产者线程
    // 使用 move 将 items 和 tx 的所有权转移进子线程
    thread::spawn(move || {
        for item in items {
            // send() 会将数据发送到频道中
            // 如果接收端已经关闭，send 会返回错误，这里简单处理
            tx.send(item).expect("Failed to send message");
        }
        // 当这个闭包结束时，tx 会离开作用域并被自动 drop
        // 这对接收端非常重要，因为它是“停止接收”的信号
    });

    // 3. 在主线程中接收所有消息
    let mut received = Vec::new();
    
    // 使用 iter() 或直接在 rx 上进行循环
    // 当所有的 Sender 都被 drop 且频道为空时，迭代器会自动停止
    for msg in rx {
        received.push(msg);
    }

    received
}

/// Create `n_producers` producer threads, each sending a message in format `"msg from {id}"`.
/// Collect all messages, sort them lexicographically, and return.
///
/// Hint: Use `tx.clone()` to create multiple senders. Note that the original tx must also be dropped.
pub fn multi_producer(n_producers: usize) -> Vec<String> {
    // 1. 创建频道
    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];

    for id in 0..n_producers {
        // 2. 为每个生产者克隆一个发送端
        let tx_clone = tx.clone();
        
        let handle = thread::spawn(move || {
            let msg = format!("msg from {}", id);
            tx_clone.send(msg).expect("Failed to send message");
            // tx_clone 在这里离开作用域并被 drop
        });
        handles.push(handle);
    }

    // 3. 【核心步骤】丢弃原始的 tx
    // 此时主线程手里还拿着最初创建频道时的那个 tx。
    // 如果不 drop 它，rx 会一直等下去，因为理论上主线程还能用这个 tx 发消息。
    drop(tx);

    // 4. 收集所有消息
    let mut messages: Vec<String> = rx.into_iter().collect();

    // 5. 按字典序排序
    messages.sort();

    // 确保所有线程都执行完毕（虽然 collect 已经保证了数据收完）
    for handle in handles {
        handle.join().unwrap();
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_send_recv() {
        let items = vec!["hello".into(), "world".into(), "rust".into()];
        let result = simple_send_recv(items.clone());
        assert_eq!(result, items);
    }

    #[test]
    fn test_simple_empty() {
        let result = simple_send_recv(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_multi_producer() {
        let result = multi_producer(3);
        assert_eq!(
            result,
            vec![
                "msg from 0".to_string(),
                "msg from 1".to_string(),
                "msg from 2".to_string(),
            ]
        );
    }

    #[test]
    fn test_multi_producer_single() {
        let result = multi_producer(1);
        assert_eq!(result, vec!["msg from 0".to_string()]);
    }
}
