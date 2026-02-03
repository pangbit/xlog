use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(100);

        // 前 100 次应该成功
        for _ in 0..100 {
            assert!(limiter.try_acquire());
        }

        // 第 101 次应该失败
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new(10);

        // 消耗限额
        for _ in 0..10 {
            limiter.try_acquire();
        }

        // 等待窗口重置
        std::thread::sleep(Duration::from_millis(1100));

        // 应该可以再次获取
        assert!(limiter.try_acquire());
    }
}

/// 轻量级速率限制器 - 滑动窗口算法
pub struct RateLimiter {
    max_per_second: u32,
    counter: AtomicU32,
    window_start: AtomicU64,
}

impl RateLimiter {
    pub fn new(max_per_second: u32) -> Self {
        Self {
            max_per_second,
            counter: AtomicU32::new(0),
            window_start: AtomicU64::new(now_nanos()),
        }
    }

    /// 尝试获取许可
    #[inline]
    pub fn try_acquire(&self) -> bool {
        let now = now_nanos();
        let window = self.window_start.load(Ordering::Relaxed);

        // 检查是否需要重置窗口（超过1秒）
        if now.saturating_sub(window) >= 1_000_000_000 {
            // 尝试重置窗口
            if self.window_start.compare_exchange(
                window,
                now,
                Ordering::Release,
                Ordering::Relaxed
            ).is_ok() {
                self.counter.store(0, Ordering::Release);
            }
        }

        // 尝试递增计数器
        let count = self.counter.fetch_add(1, Ordering::AcqRel);
        count < self.max_per_second
    }
}

#[inline]
fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
