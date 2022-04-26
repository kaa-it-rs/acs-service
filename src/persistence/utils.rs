use crate::persistence::error::Error;
use mongodb::error::{ErrorKind, WriteFailure};

pub(super) fn check_already_exists(e: mongodb::error::Error) -> anyhow::Error {
    let exists = match e.kind.as_ref() {
        ErrorKind::Write(ref failure) => match failure {
            WriteFailure::WriteError(ref we) => match we.code {
                11000 => true,
                _ => false,
            },
            _ => false,
        },
        _ => false,
    };

    if exists {
        return Error::AlreadyExistsError.into();
    }

    return e.into();
}
