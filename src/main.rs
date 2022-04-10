use {
  actix_cors::Cors,
  actix_http::{KeepAlive, StatusCode},
  actix_web::{
    get, middleware, post,
    web::{self, Bytes},
    App, HttpResponse, HttpServer,
  },
  error::Failure,
  futures_util::StreamExt,
  std::sync::atomic::{AtomicU64, Ordering},
};

mod error;

// NOTE: a static global atomic performs about the same as web::Data<Count>
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

async fn read_payload_static(
  mut payload: web::Payload,
) -> actix_web::Result<Option<([u8; 3], usize)>> {
  let mut buf = [0u8; 3];
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

fn write_payload(count: u64) -> [u8; 20] {
  use std::io::Write;
  let mut buf = [0u8; 20];
  write!(&mut buf[..], "{count}").expect("Buffer is the wrong size");
  buf
}

#[post("/")]
async fn submit(body: web::Payload, count: web::Data<Count>) -> actix_web::Result<HttpResponse> {
  // maximum value = 500
  // which is 3 digits
  let (raw, len) = read_payload_static(body)
    .await
    .internal()?
    .failed(StatusCode::BAD_REQUEST)?;

  let value = std::str::from_utf8(&raw[0..len])
    .failed(StatusCode::BAD_REQUEST)?
    .parse::<u16>()
    .failed(StatusCode::BAD_REQUEST)? as u64;

  if value > 500 {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let old_count = count.get_ref().add(value);
  let new_count = old_count + value;

  let response = write_payload(new_count);
  Ok(HttpResponse::Ok().body(Bytes::copy_from_slice(&response[..])))
}

#[get("/")]
async fn sync(count: web::Data<Count>) -> actix_web::Result<HttpResponse> {
  let count = count.get_ref().get();
  Ok(HttpResponse::Ok().body(format!("{count}")))
}

pub fn init_logger() {
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
  .backlog(1024)
  .keep_alive(KeepAlive::Os)
  .client_request_timeout(std::time::Duration::from_millis(200))
  .client_disconnect_timeout(std::time::Duration::from_millis(400))
  .bind(&addr)?
  .run()
  .await
}
