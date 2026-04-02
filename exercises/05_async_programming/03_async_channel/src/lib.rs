//! # Async Channel
//!
//! In this exercise, you will use `tokio::sync::mpsc` async channels to implement producer-consumer pattern.
//!
//! ## Concepts
//! - `tokio::sync::mpsc::channel` creates bounded async channels
//! - Async `send` and `recv`
//! - Channel closing mechanism (receiver returns None after all senders are dropped)

use tokio::sync::mpsc;

/// Async producer-consumer:
/// - Create a producer task that sends each element from items sequentially
/// - Create a consumer task that receives all elements and collects them into Vec for return
///
/// Hint: Set channel capacity to items.len().max(1)
pub async fn producer_consumer(items: Vec<String>) -> Vec<String> {
    // 1. 创建通道：tx 是发送端 (Sender)，rx 是接收端 (Receiver)
    let (tx, mut rx) = mpsc::channel(100); // 100 是缓冲区大小

    // 2. 生成生产者任务
    spawn(async move {
        for item in items {
            // 发送数据到通道
            let _ = tx.send(item).await;
        }
        // 当 tx 离开作用域被 drop 时，通道会自动关闭
    });

    // 3. 生成消费者任务
    let consumer_handle = spawn(async move {
        let mut results = Vec::new();
        // 只要通道没关闭且有数据，就会持续接收
        while let Some(msg) = rx.recv().await {
            results.push(msg);
        }
        results
    });

    // 4. 等待消费者干完活并返回结果
    consumer_handle.await
}

/// Fan‑in pattern: multiple producers, one consumer.
/// Create `n_producers` producers, each sending `"producer {id}: message"`.
/// Consumer collects all messages, sorts them, and returns.
pub async fn fan_in(n_producers: usize) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(100);

    // 1. 派发多个生产者
    for id in 0..n_producers {
        // 重要：tx 是可以克隆的！每个生产者拿一个克隆副本
        let tx_clone = tx.clone();
        spawn(async move {
            let msg = format!("producer {id}: message");
            let _ = tx_clone.send(msg).await;
        });
    }

    // 2. 关键点：丢弃原始的 tx！
    // 只要还有一个 tx 在世，rx.recv() 就会一直等下去。
    // 我们已经把克隆体分发给了所有生产者，所以这里的“本体”必须死掉，
    // 这样当所有生产者任务结束（它们的克隆体被释放）后，通道才会真正关闭。
    drop(tx);

    // 3. 消费者收集数据
    let mut results = Vec::new();
    while let Some(msg) = rx.recv().await {
        results.push(msg);
    }

    // 4. 排序并返回
    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_producer_consumer() {
        let items = vec!["hello".into(), "async".into(), "world".into()];
        let result = producer_consumer(items.clone()).await;
        assert_eq!(result, items);
    }

    #[tokio::test]
    async fn test_producer_consumer_empty() {
        let result = producer_consumer(vec![]).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fan_in() {
        let result = fan_in(3).await;
        assert_eq!(
            result,
            vec![
                "producer 0: message",
                "producer 1: message",
                "producer 2: message",
            ]
        );
    }

    #[tokio::test]
    async fn test_fan_in_single() {
        let result = fan_in(1).await;
        assert_eq!(result, vec!["producer 0: message"]);
    }
}
