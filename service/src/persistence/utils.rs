use crate::persistence::error::Error;
use mongodb::error::{ErrorKind, WriteFailure};

pub(super) fn check_already_exists(e: mongodb::error::Error) -> anyhow::Error {
    let exists = match e.kind.as_ref() {
        ErrorKind::Write(WriteFailure::WriteError(ref we)) => matches!(we.code, 11000),
        _ => false,
    };

    if exists {
        return Error::AlreadyExistsError.into();
    }

    e.into()
}
