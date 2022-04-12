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

mod error;

#[cfg(feature = "backend-file")]
pub mod mmap;

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

pub mod utils {
  pub fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
    if s.len() > 7 {
      return Err(format!(
        "Invalid duration string. Expected a number followed by 'ms', 's', or 'm', but received '{}'", s
      ));
    }
    let mut num = 0;
    let mut bytes = s.bytes().take(7).peekable();
    while let Some(b'0'..=b'9') = bytes.peek() {
      let c = bytes.next().unwrap();
      num = (num as u32)
        .saturating_mul(10)
        .saturating_add((c - b'0') as u32);
    }

    let length = bytes.map(|b| b as char).collect::<String>();

    if num > u16::MAX as u32 {
      return Err(format!(
        "Provided duration is too large. Expected a number <= {}, but got {}",
        u16::MAX,
        num
      ));
    }

    let num = num as u64;
    match &length[..] {
      "ms" | "" => Ok(std::time::Duration::from_millis(num)),
      "s" => Ok(std::time::Duration::from_secs(num)),
      "m" => Ok(std::time::Duration::from_secs(num * 60)),
      _ => Err(format!(
        "Unknown duration unit. Expected 'ms', 's', or 'm', but received '{}'",
        length
      )),
    }
  }
}

#[cfg(test)]
mod tests {
  use {super::utils::*, std::time::Duration};

  #[test]
  fn test_duration_parsing() {
    let tests = vec![
      "", "1ms", "1s", "2m", "0m", "22222s", "21818m", "65535ms", "ms", "m", "s",
    ];
    let expected = vec![
      Duration::from_millis(0),
      Duration::from_millis(1),
      Duration::from_secs(1),
      Duration::from_secs(2 * 60),
      Duration::from_secs(0 * 60),
      Duration::from_secs(22222),
      Duration::from_secs(21818 * 60),
      Duration::from_millis(65535),
      Duration::from_millis(0),
      Duration::from_millis(0),
      Duration::from_millis(0),
    ];

    for (i, (t, e)) in tests.iter().zip(expected).enumerate() {
      assert_eq!(parse_duration(t), Ok(e), "Test {} failed", i);
    }
  }

  #[test]
  fn test_duration_parsing_errors() {
    let tests = vec![
      "fldaksjfs",
      "11111111111111111111111111111111111",
      "65536",
      "6 5 5 3 5",
      "644....,/,/??>.....,",
      "ssssssssssssssssssssssssssssssssssssssss",
      "mmmmmmmmmmm",
      "mmmmmmmmmssssssssssss",
      "65535mss",
      "65535sm",
    ];

    for (i, t) in tests.iter().enumerate() {
      assert!(dbg!(parse_duration(t)).is_err(), "Test {} failed", i);
    }
  }
}
