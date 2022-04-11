#![feature(core_intrinsics)]
extern crate benchmarks;

use std::time::Duration;

use reqwest::{Body, Method, Request, Url};

#[tokio::main]
async fn main() {
  let args = std::env::args().skip(1).take(2).collect::<Vec<_>>();
  if args.len() != 2 {
    panic!("Must provide [METHOD] [URL]");
  }

  let method = args[0]
    .parse::<Method>()
    .expect("Expected a valid HTTP method");
  let url = args[1].parse::<Url>().expect("Expected a valid URL");

  eprintln!("START {method} {url}");

  let mut request = Request::new(method, url);
  let bytes = Box::leak(Box::new(500u16.to_le_bytes()));
  *request.body_mut() = Some(Body::from(bytes as &[u8]));
  let headers = reqwest::header::HeaderMap::new();

  let client = &*Box::leak(Box::new(
    reqwest::Client::builder()
      .default_headers(headers)
      .build()
      .unwrap(),
  ));
  let request = &*Box::leak(Box::new(
    request.try_clone().expect("Request must be clonable"),
  ));

  const N_CPUS: usize = 1;
  benchmarks::run!(
    C = 1024 * N_CPUS,
    report_freq = Duration::from_millis(1000),
    send = client
      .execute(unsafe { request.try_clone().unwrap_unchecked() })
      .await
      .map(|res| res.status().as_u16())
      .unwrap_or(500)
      < 400,
    timeout = Duration::from_secs(60)
  );
}
