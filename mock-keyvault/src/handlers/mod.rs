use std::fmt::Display;

use poem::{
    error::ResponseError,
    http::StatusCode,
};

pub mod authorize;
pub mod secrets;

#[derive(Debug, thiserror::Error)]
pub struct UnauthorizedError;

impl Display for UnauthorizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "unauthorized")
    }
}

impl ResponseError for UnauthorizedError {
    fn status(&self) -> StatusCode {
        StatusCode::UNAUTHORIZED
    }
}
