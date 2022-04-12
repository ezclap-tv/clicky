use std::io::Cursor;

use {
  crate::Count,
  actix_web::web,
  std::{
    fs::{self, File},
    io::Read,
    time::Duration,
  },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("{0}")]
  Io(#[from] std::io::Error),
  #[error("{0}")]
  ParseInt(#[from] std::num::ParseIntError),
  #[error("{0}")]
  Utf8(#[from] std::str::Utf8Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Memory mapped file storage
pub struct FileStorage {
  file: File,
  sync_frequency: Duration,
}

const MIN_DURATION: Duration = Duration::new(1, 0);

fn parse_file_contents(file: &mut File) -> Result<u64> {
  use std::io::{Seek, SeekFrom};
  file.seek(SeekFrom::Start(0))?;
  let mut buf = [0; 20];
  let len = file.read(&mut buf)?;
  let str = std::str::from_utf8(&buf[..len])?;
  if str.is_empty() {
    Ok(0)
  } else {
    Ok(str.parse()?)
  }
}

fn write_file_contents(file: &mut File, value: u64) -> Result<()> {
  use std::io::{Seek, SeekFrom, Write};
  file.seek(SeekFrom::Start(0))?;
  let mut buf = Cursor::new([0u8; 20]);
  write!(&mut buf, "{value}")?;
  let (pos, buf) = (buf.position() as usize, buf.into_inner());
  file.write_all(&buf[..pos])?;
  Ok(())
}

impl FileStorage {
  pub fn from_env() -> Result<Self> {
    let path = std::env::var("CLICKY_COUNTER_FILE").unwrap_or_else(|_| "clicky.txt".into());
    let sync_frequency = match std::env::var("CLICKY_SYNC_FREQUENCY").ok() {
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
    };

    let file = fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .open(&path)?;

    Ok(Self {
      file,
      sync_frequency,
    })
  }

  pub fn install(self, counter: web::Data<Count>) -> Result<()> {
    let Self {
      mut file,
      sync_frequency,
    } = self;

    counter.set(parse_file_contents(&mut file)?);

    std::thread::spawn(move || {
      let mut previous_count = u64::MAX;
      loop {
        let count = counter.get();
        if count != previous_count {
          if let Err(e) = write_file_contents(&mut file, count) {
            panic!("Failed to persist the counter at {count}: {e}");
          } else {
            log::info!("Persisted counter at {count}");
          }
          previous_count = count;
        }

        // Note: thread::sleep utilizes the OS scheduler so we don't need to yield/do anything else
        std::thread::sleep(sync_frequency);
      }
    });

    Ok(())
  }
}
