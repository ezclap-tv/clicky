use std::{fs::File, io::Read};

use actix_web::web;

pub struct MmapBackend {
  file: std::fs::File,
  sync_frequency: std::time::Duration,
}

impl MmapBackend {
  pub fn new(file: File, sync_frequency: std::time::Duration) -> Self {
    Self {
      file,
      sync_frequency,
    }
  }

  pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
    let path = std::env::var("CLICKY_COUNTER_FILE").unwrap_or_else(|_| "clicky.txt".into());
    let mut sync_frequency = crate::utils::parse_duration(
      &std::env::var("CLICKY_SYNC_FREQUENCY").unwrap_or_else(|_| "1s".into())[..],
    )?;
    if sync_frequency.as_millis() == 0 {
      sync_frequency = std::time::Duration::from_millis(1000);
    }

    let file = std::fs::OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .append(true)
      .open(&path)?;

    Ok(Self::new(file, sync_frequency))
  }

  pub fn install(self, counter: web::Data<crate::Count>) -> Result<(), Box<dyn std::error::Error>> {
    let Self {
      mut file,
      sync_frequency,
    } = self;

    let mut buf = [0; 20];
    let read = file.read(&mut buf)?;
    file.set_len(20)?; // u64 is 20 digits max

    let previous_count = buf[..read].iter().map(|&b| b as char).collect::<String>();
    let previous_count = if previous_count.len() > 0 {
      previous_count.parse::<u64>().map_err(|e| {
        log::error!(
          "[CLICKY] Non-empty storage file didn't contain a number: `{}`",
          previous_count
        );
        e
      })?
    } else {
      0
    };

    counter.set(previous_count);

    let mut mmap = unsafe { memmap2::MmapMut::map_mut(&file) }?;
    std::thread::spawn(move || {
      let mut previous_count = u64::MAX;
      loop {
        use std::io::Write;

        let count = counter.get();
        if count != previous_count {
          if let Err(e) = (&mut mmap[..20]).write(format!("{:020}", count).as_bytes()) {
            log::error!("[CLICKY] Failed to persist the counter at {}: {}", count, e);
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
