mod error;

use actix_cors::Cors;
use actix_http::StatusCode;
use actix_web::{get, middleware, post, web, App, HttpResponse, HttpServer};
use error::Failure;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Atomic counter
#[derive(Default)]
struct Count(AtomicU64);
impl Count {
  /// Returns the current value
  fn get(&self) -> u64 {
    self.0.load(Ordering::SeqCst)
  }

  /// Adds `value` to `self`, returning the previous value
  fn add(&self, value: u64) -> u64 {
    self.0.fetch_add(value, Ordering::SeqCst)
  }
}

async fn read_payload_static<const SIZE: usize>(
  mut payload: web::Payload,
) -> actix_web::Result<Option<([u8; SIZE], usize)>> {
  let mut buf = [0u8; SIZE];
  let mut read_bytes = 0;
  while let Some(data) = payload.next().await {
    let data = data?;
    if data.is_empty() || data.len() > buf.len() - read_bytes {
      return Ok(None);
    }
    buf[read_bytes..data.len()].copy_from_slice(&data);
    read_bytes += data.len();
  }
  Ok(Some((buf, read_bytes)))
}

#[post("/")]
async fn submit(body: web::Payload, count: web::Data<Count>) -> actix_web::Result<HttpResponse> {
  // maximum value = 500
  // which is 3 digits
  let (raw, len) = read_payload_static::<3>(body)
    .await
    .internal()?
    .failed(StatusCode::BAD_REQUEST)?;
  log::info!("{raw:?}");
  let value = std::str::from_utf8(&raw[0..len])
    .failed(StatusCode::BAD_REQUEST)?
    .parse::<u16>()
    .failed(StatusCode::BAD_REQUEST)? as u64;
  if value > 500 {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let old_count = count.get_ref().add(value);
  let new_count = old_count + value;
  log::info!("Updated count: {new_count}");

  Ok(HttpResponse::Ok().body(format!("{new_count}")))
}

#[get("/")]
async fn sync(count: web::Data<Count>) -> actix_web::Result<HttpResponse> {
  let count = count.get_ref().get();
  Ok(HttpResponse::Ok().body(format!("{count}")))
}

fn init_logger() {
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "info,actix_web=debug"); // actix_web=debug enables error logging
  }
  env_logger::init();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  init_logger();
  let addr = std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".into());
  let count = web::Data::new(Count::default());
  // TODO: rate limiting (20 tokens every second)
  HttpServer::new(move || {
    App::new()
      .app_data(web::Data::clone(&count))
      .wrap(
        Cors::default()
          .allow_any_origin()
          .allowed_methods(vec!["GET", "POST"])
          .allow_any_header()
          .max_age(3600),
      )
      .wrap(middleware::Logger::default())
      .service(sync)
      .service(submit)
  })
  .bind(&addr)?
  .run()
  .await
}
