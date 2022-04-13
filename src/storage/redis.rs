use {crate::Count, actix_web::web, std::time::Duration};

use redis::Commands;

const ERRORS_TO_RECONNECT: usize = 3;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("{0}")]
  Io(#[from] std::io::Error),
  #[error("{0}")]
  ReidError(#[from] redis::RedisError),
  #[error("{0}")]
  ParseInt(#[from] std::num::ParseIntError),
  #[error("{0}")]
  Utf8(#[from] std::str::Utf8Error),
}

pub type Result<T> = std::result::Result<T, Error>;
/// Memory mapped file storage
pub struct RedisStorage {
  client: redis::Client,
  sync_frequency: Duration,
}

const REDIS_KEY: &str = "CLICKY_COUNTER";

impl RedisStorage {
  pub fn from_env() -> Result<Self> {
    let url = std::env::var("CLICKY_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());

    let sync_frequency = super::parse_sync_frequency();

    // Doesn't actually connect, only validates the settings
    log::info!("Using Redis at {url}");
    let client = redis::Client::open(url)?;

    Ok(Self {
      client,
      sync_frequency,
    })
  }

  pub fn install(self, counter: web::Data<Count>) -> Result<()> {
    let Self {
      client,
      sync_frequency,
    } = self;

    // Obtain the current count on the main thread so the API doesn't erroneously respond with a zero count
    let mut conn = client.get_connection().map_err(|e| {
      log::error!("Failed to connect to Redis");
      e
    })?;
    let count = conn.get::<_, Option<u64>>(REDIS_KEY)?.unwrap_or(0);
    counter.add(count);

    std::thread::spawn(move || {
      let mut conn = conn;
      let mut tracker = CountTracker {
        local_count: counter,
        previous_remote_count: count,
      };
      let mut n_errors = 0;
      loop {
        let res: redis::RedisResult<u64> =
          redis::transaction(&mut conn, &[REDIS_KEY], |con, pipe| {
            let mut remote_count: u64 = con
              .get::<_, Option<u64>>(REDIS_KEY)?
              .unwrap_or_else(|| tracker.previous_remote_count);
            remote_count = tracker.get_new_count(remote_count);
            pipe.set(REDIS_KEY, remote_count).query(con)?;
            Ok(Some(remote_count))
          });

        match res {
          Ok(count) => {
            log::info!("Persisted counter at {count}");
          }
          Err(e) => {
            // Don't panic since it might've been a random network hiccup
            log::error!(
              "Failed to persist the counter at {} (attempt {}): {}",
              tracker.local_count.get(),
              n_errors,
              e
            );
            n_errors += 1;

            // Try to re-create the connection if we're failing repeatedly.
            if n_errors % ERRORS_TO_RECONNECT == 0 {
              match client.get_connection() {
                Ok(new_conn) => {
                  n_errors = 0;
                  conn = new_conn;
                }
                Err(e) => {
                  log::error!(
                    "Failed to reconnect {} times: {}",
                    n_errors / ERRORS_TO_RECONNECT,
                    e
                  );
                }
              }
            }
          }
        }

        std::thread::sleep(sync_frequency);
      }
    });

    Ok(())
  }
}

struct CountTracker {
  local_count: web::Data<Count>,
  previous_remote_count: u64,
}

impl CountTracker {
  pub fn get_new_count(&mut self, mut remote_count: u64) -> u64 {
    // NOTE: The remote count can only ever go down if the user manually changes it,
    // so we could accept it as the new ground truth, and update the local counter -- however,
    // the invariant we want to maintain is that the counter never goes down, so instead, we
    // pretend that the remote count didn't change and submit our local count normally.
    if std::intrinsics::unlikely(remote_count < self.previous_remote_count) {
      remote_count = self.previous_remote_count;
    };

    let remote_contribution = remote_count - self.previous_remote_count;
    let local_count = self.local_count.add(remote_contribution);
    let local_contribution = local_count - self.previous_remote_count;
    let remote_count = remote_count + local_contribution;
    self.previous_remote_count = remote_count;
    remote_count
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_count_tracker_with_different_counts() {
    let local_count = web::Data::new(Count::default());
    let mut remote_count = 0;

    let mut tracker = CountTracker {
      local_count: local_count.clone(),
      previous_remote_count: remote_count,
    };

    local_count.add(37);
    remote_count += 3;

    let remote_count = tracker.get_new_count(remote_count);
    assert_eq!(remote_count, local_count.get());
  }

  #[test]
  fn test_count_tracker_with_same_counts() {
    let local_count = web::Data::new(Count::default());
    let mut remote_count = 0;

    let mut tracker = CountTracker {
      local_count: local_count.clone(),
      previous_remote_count: remote_count,
    };

    local_count.add(100);
    remote_count += 100;

    let remote_count = tracker.get_new_count(remote_count);
    assert_eq!(remote_count, local_count.get());
  }

  #[test]
  fn test_remote_count_going_down() {
    let local_count = web::Data::new(Count::default());
    let mut remote_count = 97;

    let mut tracker = CountTracker {
      local_count: local_count.clone(),
      previous_remote_count: remote_count,
    };

    local_count.add(97);
    remote_count = 50; // count went down -- this requires manual user intervention
    local_count.add(3);

    let remote_count = tracker.get_new_count(remote_count);
    assert_eq!(remote_count, 100);
    assert_eq!(remote_count, local_count.get());
  }

  #[test]
  fn test_remote_concurrent_updates() {
    let mut remote_count = 0;

    let mut tracker_a = CountTracker {
      local_count: web::Data::new(Count::default()),
      previous_remote_count: remote_count,
    };
    let mut tracker_b = CountTracker {
      local_count: web::Data::new(Count::default()),
      previous_remote_count: remote_count,
    };

    // R -> 100
    // A -> 100
    // B == 0
    tracker_a.local_count.add(100);
    remote_count = tracker_a.get_new_count(remote_count);
    assert_eq!(remote_count, 100);
    assert_eq!(remote_count, tracker_a.local_count.get());

    // R -> 100
    // A == 100
    // B -> 100
    let remote_count_1 = tracker_b.get_new_count(remote_count);
    assert_eq!(remote_count, remote_count_1);
    assert_eq!(remote_count, tracker_b.local_count.get());

    // R -> 200
    // A == 100
    // B -> 200
    tracker_b.local_count.add(100);
    let remote_count_2 = tracker_b.get_new_count(remote_count);
    assert_eq!(remote_count_2, 200);
    assert_eq!(remote_count_2, tracker_b.local_count.get());
    remote_count = remote_count_2;

    // R == 200
    // A -> 200
    // B == 200
    let remote_count_3 = tracker_a.get_new_count(remote_count);
    assert_eq!(remote_count, remote_count_3);
    assert_eq!(remote_count, tracker_a.local_count.get());

    // R == 200
    // A == 200
    // B == 200
    assert_eq!(tracker_b.local_count.get(), tracker_a.local_count.get());
  }
}
