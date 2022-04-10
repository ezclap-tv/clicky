#![feature(core_intrinsics)]
use std::time::Duration;

pub use std::intrinsics;
pub use tokio::sync::Semaphore;

#[macro_export]
macro_rules! run {
    (C = $concurrency:expr, report_freq = $d:expr, send = $send:expr) => {
        $crate::run!(
            C = $concurrency,
            report_freq = $d,
            send = $send,
            __timeout = loop
        )
    };
    (C = $concurrency:expr, report_freq = $d:expr, send = $send:expr, timeout = $timeout_duration:expr) => {
        let start = std::time::Instant::now();
        let timeout = $timeout_duration;
        $crate::run!(
            C = $concurrency,
            report_freq = $d,
            send = $send,
            __timeout = while start.elapsed() < timeout
        )
    };
    (C = $concurrency:expr, report_freq = $d:expr, send = $send:expr, __timeout = $($timeout:tt)*) => {{
        const CONCURRENCY: usize = $concurrency;
        static SEM: $crate::Semaphore = $crate::Semaphore::const_new(CONCURRENCY);
        static COUNTER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        let report_frequency: core::time::Duration = $d;

        let mut prev_count = 0;
        let mut time = std::time::Instant::now();
        let total_time = std::time::Instant::now();

        $($timeout)* {
            let permit = SEM.acquire().await.unwrap();
            tokio::spawn({
                async move {
                    let result: bool = $send;
                    std::mem::drop(permit);
                    if std::intrinsics::likely(result) {
                        COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                    } else {
                        return;
                    }
                }
            });

            if time.elapsed() > report_frequency {
                let count = COUNTER.load(core::sync::atomic::Ordering::Relaxed);
                eprintln!(
                    "Total requests: {}. Requests in the last {}ms: {} [{:.3}s total] [{} avg. RPS]",
                    count,
                    time.elapsed().as_millis(),
                    count - prev_count,
                    total_time.elapsed().as_secs_f64(),
                    count as f64 / total_time.elapsed().as_secs_f64()
                );
                prev_count = count;
                time = std::time::Instant::now();
            }
        }
        #[allow(unreachable_code)]
        COUNTER.load(core::sync::atomic::Ordering::Relaxed)
    }};
}

#[inline]
pub async fn http_client(
  request: reqwest::Request,
  headers: reqwest::header::HeaderMap,
) -> Result<usize, Box<dyn std::error::Error + 'static>> {
  let client = &*Box::leak(Box::new(
    reqwest::Client::builder()
      .default_headers(headers)
      .build()
      .unwrap(),
  ));
  let request = &*Box::leak(Box::new(
    request.try_clone().expect("Request must be clonable"),
  ));
  #[allow(unused)]
  let res = run!(
    C = 1024,
    report_freq = Duration::from_millis(1000),
    send = client
      .execute(unsafe { request.try_clone().unwrap_unchecked() })
      .await
      .map(|res| res.status().as_u16())
      .unwrap_or(500)
      < 400
  );
  unsafe {
    Box::from_raw(&mut *request);
    Box::from_raw(&mut *client);
  }
  Ok(res)
}
