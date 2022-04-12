use {
  crate::Count,
  actix_web::web,
  std::{
    fs::{self, File},
    io::Read,
    time::Duration,
  },
};

/// Memory mapped file storage
pub struct FileStorage {
  file: File,
  sync_frequency: Duration,
}

const MIN_DURATION: Duration = Duration::new(1, 0);

impl FileStorage {
  pub fn from_env() -> std::io::Result<Self> {
    let path = std::env::var("CLICKY_COUNTER_FILE").unwrap_or_else(|_| "clicky.bin".into());
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

  pub fn install(self, counter: web::Data<Count>) -> std::io::Result<()> {
    let Self {
      mut file,
      sync_frequency,
    } = self;

    let mut buf = [0; 8];
    let _ = file.read(&mut buf)?;
    file.set_len(8)?;
    counter.set(u64::from_le_bytes(buf));

    let mut mmap = unsafe { memmap2::MmapMut::map_mut(&file) }?;
    std::thread::spawn(move || {
      let mut previous_count = u64::MAX;
      loop {
        use std::io::Write;

        let count = counter.get();
        if count != previous_count {
          if let Err(e) = (&mut mmap[..]).write_all(&count.to_le_bytes()) {
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
