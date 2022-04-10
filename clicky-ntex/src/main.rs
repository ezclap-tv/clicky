mod error;

use {
  error::{Error, Failure},
  futures_util::StreamExt,
  ntex::{
    http::{KeepAlive, StatusCode},
    util::Bytes,
    web::{
      self,
      types::{Payload, State},
      App, HttpResponse, HttpServer,
    },
  },
  ntex_cors::Cors,
  std::sync::atomic::{AtomicU64, Ordering},
};

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
  mut payload: ntex::web::types::Payload,
) -> Result<Option<([u8; SIZE], usize)>, Error> {
  let mut buf = [0u8; SIZE];
  let mut read_bytes = 0;
  while let Some(data) = payload.next().await {
    let data = data.failed(StatusCode::BAD_REQUEST)?;
    if data.is_empty() || data.len() > buf.len() - read_bytes {
      return Ok(None);
    }
    buf[read_bytes..data.len()].copy_from_slice(&data);
    read_bytes += data.len();
  }
  Ok(Some((buf, read_bytes)))
}

#[web::post("/")]
async fn submit(body: Payload, count: State<Count>) -> Result<HttpResponse, Error> {
  // NOTE: transmute vs parse doesn't make a difference

  // let (raw, len) = read_payload_static::<2>(body)
  //   .await
  //   .internal()
  //   .and_then(|i| i.failed(StatusCode::BAD_REQUEST))?;
  // let value = u16::from_be_bytes(raw);

  let (raw, len) = read_payload_static::<3>(body)
    .await
    .internal()
    .and_then(|i| i.failed(StatusCode::BAD_REQUEST))?;

  let value = std::str::from_utf8(&raw[0..len])
    .failed(StatusCode::BAD_REQUEST)?
    .parse::<u16>()
    .failed(StatusCode::BAD_REQUEST)? as u64;

  if value > 500 {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let old_count = count.get_ref().add(value as u64);
  let new_count = old_count + value as u64;

  // NOTE: Bytes doesn't allocate if the payload is <=30 bytes
  Ok(HttpResponse::Ok().body(Bytes::copy_from_slice(&new_count.to_le_bytes())))
}

#[web::get("/")]
async fn sync(count: State<Count>) -> HttpResponse {
  let count = count.get_ref().get();
  HttpResponse::Ok().body(format!("{count}"))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
  let addr = std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".into());
  let count = State::new(Count::default());
  HttpServer::new(move || {
    App::new()
      .app_state(State::clone(&count))
      .wrap(
        Cors::new()
          .allowed_methods(vec!["GET", "POST"])
          .max_age(3600)
          .finish(),
      )
      .service((sync, submit))
  })
  .backlog(2048)
  .keep_alive(KeepAlive::Os)
  .bind(&addr)?
  .run()
  .await
}
