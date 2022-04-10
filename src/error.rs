use {
  actix_http::StatusCode,
  actix_web::{error::ResponseError, HttpResponse},
};

#[derive(Debug, Clone)]
pub struct Error {
  pub code: StatusCode,
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.code)
  }
}

impl std::error::Error for Error {}

impl ResponseError for Error {
  fn status_code(&self) -> StatusCode {
    self.code
  }

  fn error_response(&self) -> HttpResponse {
    HttpResponse::build(self.status_code()).finish()
  }
}

pub trait Failure<T> {
  fn failed(self, code: StatusCode) -> std::result::Result<T, Error>;
  fn internal(self) -> std::result::Result<T, Error>;
}

impl<T, E: std::fmt::Debug> Failure<T> for std::result::Result<T, E> {
  fn failed(self, code: StatusCode) -> std::result::Result<T, Error> {
    self.map_err(|e| {
      log::error!("Discarded error: {:?}", e);
      Error { code }
    })
  }

  fn internal(self) -> std::result::Result<T, Error> {
    self.map_err(|e| {
      log::error!("Discarded error: {:?}", e);
      Error {
        code: StatusCode::INTERNAL_SERVER_ERROR,
      }
    })
  }
}

impl<T> Failure<T> for std::option::Option<T> {
  fn failed(self, code: StatusCode) -> std::result::Result<T, Error> {
    self.ok_or(Error { code })
  }

  fn internal(self) -> std::result::Result<T, Error> {
    self.ok_or(Error {
      code: StatusCode::INTERNAL_SERVER_ERROR,
    })
  }
}
