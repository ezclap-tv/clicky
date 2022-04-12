use {
  actix_http::StatusCode,
  actix_web::{
    get, post,
    web::{self, Bytes},
    HttpResponse,
  },
  error::Failure,
  futures_util::StreamExt,
  std::{
    io::Cursor,
    sync::atomic::{AtomicU64, Ordering},
  },
};

pub mod error;
pub mod storage;

/// Atomic counter
#[derive(Default)]
pub struct Count(AtomicU64);
impl Count {
  /// Returns the current value
  pub fn get(&self) -> u64 {
    self.0.load(Ordering::SeqCst)
  }

  /// Adds `value` to `self`, returning the previous value
  pub fn add(&self, value: u64) -> u64 {
    self.0.fetch_add(value, Ordering::SeqCst)
  }

  /// Sets `self` to `value`, returning the previous value
  pub fn set(&self, value: u64) -> u64 {
    self.0.swap(value, Ordering::SeqCst)
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

fn write_payload(count: u64) -> ([u8; 20], usize) {
  use std::io::Write;
  let mut cursor = Cursor::new([0u8; 20]);
  write!(&mut cursor, "{count}").expect("Buffer is the wrong size");
  let pos = cursor.position() as usize;
  let buf = cursor.into_inner();
  (buf, pos)
}

#[post("/")]
pub async fn submit(
  body: web::Payload,
  count: web::Data<Count>,
) -> actix_web::Result<HttpResponse> {
  // maximum value = 500
  // which is 3 digits
  let (raw, len) = read_payload_static(body)
    .await
    .internal()?
    .failed(StatusCode::BAD_REQUEST)?;

  let value = std::str::from_utf8(&raw[..len])
    .failed(StatusCode::BAD_REQUEST)?
    .parse::<u16>()
    .failed(StatusCode::BAD_REQUEST)? as u64;

  if value > 500 {
    return Ok(HttpResponse::BadRequest().finish());
  }

  let old_count = count.get_ref().add(value);
  let new_count = old_count + value;

  let (raw, len) = write_payload(new_count);
  Ok(HttpResponse::Ok().body(Bytes::copy_from_slice(&raw[..len])))
}

#[get("/")]
pub async fn sync(count: web::Data<Count>) -> actix_web::Result<HttpResponse> {
  let count = count.get_ref().get();
  Ok(HttpResponse::Ok().body(format!("{count}")))
}
