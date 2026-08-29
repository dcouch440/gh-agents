#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::TokenBucket;

    #[tokio::test]
    async fn a_full_bucket_admits_its_capacity_without_waiting() {
        let mut bucket = TokenBucket::per_minute(60);
        let start = tokio::time::Instant::now();
        for _ in 0..60 {
            bucket.acquire().await;
        }
        assert!(
            start.elapsed() < Duration::from_millis(200),
            "a full bucket should not have slept"
        );
    }

    /// The property `brave_search` depends on: capacity 1 means a second
    /// request genuinely waits rather than bursting alongside the first.
    #[tokio::test(start_paused = true)]
    async fn a_one_per_second_bucket_serializes_requests() {
        let mut bucket = TokenBucket::per_second(1.0);

        bucket.acquire().await; // the initial token, free
        let start = tokio::time::Instant::now();
        bucket.acquire().await;

        assert!(
            start.elapsed() >= Duration::from_millis(990),
            "second request waited only {:?}",
            start.elapsed()
        );
    }

    #[tokio::test(start_paused = true)]
    async fn tokens_refill_over_time() {
        let mut bucket = TokenBucket::per_second(2.0);
        bucket.acquire().await;
        bucket.acquire().await;

        // Drained, then given a full second back: two more should be free.
        tokio::time::sleep(Duration::from_secs(1)).await;
        let start = tokio::time::Instant::now();
        bucket.acquire().await;
        bucket.acquire().await;

        assert!(start.elapsed() < Duration::from_millis(50));
    }

    /// A sub-1/s rate must still admit one request, or the bucket would never
    /// reach a whole token and every caller would hang.
    #[tokio::test(start_paused = true)]
    async fn a_rate_below_one_per_second_still_admits_a_request() {
        let mut bucket = TokenBucket::per_second(0.5);
        let start = tokio::time::Instant::now();
        bucket.acquire().await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test(start_paused = true)]
    async fn the_bucket_never_banks_more_than_its_capacity() {
        let mut bucket = TokenBucket::per_second(1.0);

        // Idle far longer than it takes to fill.
        tokio::time::sleep(Duration::from_secs(30)).await;

        bucket.acquire().await; // the one banked token
        let start = tokio::time::Instant::now();
        bucket.acquire().await; // must still wait a full second

        assert!(
            start.elapsed() >= Duration::from_millis(990),
            "idling banked more than capacity: waited {:?}",
            start.elapsed()
        );
    }
}
