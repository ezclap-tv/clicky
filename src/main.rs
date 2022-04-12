use {
  actix_http::KeepAlive,
  actix_web::{web, App, HttpServer},
};

pub fn init_logger() {
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "info,actix_web=debug"); // actix_web=debug enables error logging
  }

  if std::env::var("CLICKY_NO_LOGGING").is_err() {
    env_logger::init();
  }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  init_logger();
  let addr = std::env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".into());
  let count = web::Data::new(clicky::Count::default());

  #[cfg(feature = "backend-file")]
  {
    clicky::mmap::MmapBackend::from_env()
      .and_then(|backend| backend.install(web::Data::clone(&count)))
      .map_err(|e| {
        log::error!("{}", e);
        e
      })
      .expect("Couldn't initialize the file storage backend.");
  }
  #[cfg(feature = "backend-redis")]
  {
    compile_error!("Redis backend is not yet implemented");
  }

  HttpServer::new(move || {
    App::new()
      .app_data(web::Data::clone(&count))
      .wrap(
        actix_cors::Cors::default()
          .allow_any_origin()
          .allowed_methods(vec!["GET", "POST"])
          .allow_any_header()
          .max_age(3600),
      )
      .wrap(actix_web::middleware::Logger::default())
      .service(clicky::sync)
      .service(clicky::submit)
  })
  .backlog(1024)
  .keep_alive(KeepAlive::Os)
  .bind(&addr)?
  .run()
  .await
}
