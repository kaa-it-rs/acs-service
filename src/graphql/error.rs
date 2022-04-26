use async_graphql::*;
use std::fmt::Display;

#[derive(SimpleObject)]
pub(crate) struct InternalServerError {
    pub message: String,
}

impl<T: Display> From<T> for InternalServerError {
    fn from(e: T) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct UnauthorizedError {
    pub message: String,
}

impl From<&str> for UnauthorizedError {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct PermissionDeniedError {
    pub message: String,
}

impl From<&str> for PermissionDeniedError {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct TokenIsExpiredError {
    pub message: String,
}

impl From<&str> for TokenIsExpiredError {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct NotFoundError {
    /// Description message
    pub message: String,

    /// What is not found (for example, Role)
    pub what: String,
}

impl NotFoundError {
    pub(crate) fn new(message: &str, what: &str) -> Self {
        Self {
            message: message.to_string(),
            what: what.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct IsInvalidError {
    /// Description message
    pub message: String,

    /// What is invalid (for example, userId param)
    pub what: String,
}

impl IsInvalidError {
    pub(crate) fn new(message: &str, what: &str) -> Self {
        Self {
            message: message.to_string(),
            what: what.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct AlreadyExistsError {
    pub message: String,
}

impl<T: Display> From<T> for AlreadyExistsError {
    fn from(e: T) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub(crate) struct NoUpdateDataProvidedError {
    pub message: String,
}

impl<T: Display> From<T> for NoUpdateDataProvidedError {
    fn from(e: T) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

#[derive(Interface)]
#[graphql(field(name = "message", type = "String"))]
pub(crate) enum Error {
    InternalServerError(InternalServerError),
    UnauthorizedError(UnauthorizedError),
    PermissionDeniedError(PermissionDeniedError),
    TokenIsExpiredError(TokenIsExpiredError),
    NotFoundError(NotFoundError),
    IsInvalidError(IsInvalidError),
    AlreadyExistsError(AlreadyExistsError),
    NoUpdateDataProvidedError(NoUpdateDataProvidedError),
}
