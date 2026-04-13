use anchor_lang_v2::Error;

pub enum MultisigError {
    InvalidThreshold,
    TooManySigners,
    MissingRequiredSignature,
    LabelTooLong,
    UnauthorizedCreator,
}

impl From<MultisigError> for Error {
    fn from(e: MultisigError) -> Self {
        match e {
            MultisigError::InvalidThreshold => Error::Custom(6000),
            MultisigError::TooManySigners => Error::Custom(6001),
            MultisigError::MissingRequiredSignature => Error::Custom(6002),
            MultisigError::LabelTooLong => Error::Custom(6003),
            MultisigError::UnauthorizedCreator => Error::Custom(6004),
        }
    }
}
