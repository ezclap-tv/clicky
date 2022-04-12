use std::{
  fs::File,
  io::Read,
  sync::atomic::{AtomicBool, Ordering},
};

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

  pub fn install(
    mut self,
    counter: web::Data<crate::Count>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0; 20];
    let read = self.file.read(&mut buf)?;
    self.file.set_len(20)?; // u64 is 20 digits max

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

    let has_started = std::sync::Arc::new(AtomicBool::new(false));
    let cloned = std::sync::Arc::clone(&has_started);
    let handle = std::thread::spawn(move || {
      let mut mmap =
        unsafe { memmap::MmapMut::map_mut(&self.file).expect("Failed to mmap the counter file") };

      cloned.store(true, Ordering::SeqCst);

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
        std::thread::sleep(self.sync_frequency);
      }
    });

    // This should only pause for a brief moment -- the thread must either successfully enter the sync loop or terminate with a panic.
    while !has_started.load(Ordering::SeqCst) && !handle.is_finished() {}

    if handle.is_finished() {
      log::error!("[CLICKY] Sync thread has terminated unexpectedly");
      Err("Sync thread has terminated unexpectedly.")?;
    }

    Ok(())
  }
}
