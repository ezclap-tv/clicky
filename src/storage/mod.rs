#[cfg(feature = "backend-file")]
pub mod file;

#[cfg(feature = "backend-redis")]
pub mod redis;

#[cfg(any(feature = "backend-file", feature = "backend-redis"))]
pub fn parse_sync_frequency() -> std::time::Duration {
  use std::time::Duration;
  const MIN_DURATION: Duration = Duration::new(1, 0);

  match std::env::var("CLICKY_SYNC_FREQUENCY").ok() {
    Some(v) => humantime::parse_duration(&v)
      .map(|v| {
        if v < MIN_DURATION {
          log::warn!("Minimum for CLICKY_SYNC_FREQUENCY is 1s");
          MIN_DURATION
        } else {
          v
        }
      })
      .unwrap_or_else(|e| {
        log::warn!("Failed to parse CLICKY_SYNC_FREQUENCY, default to 1s: {e}");
        MIN_DURATION
      }),
    None => Duration::from_secs(1),
  }
}
