// Define a generic error enum that can be used throughout the croner crate
#[derive(Debug)]
pub enum CronError {
    EmptyPattern,
    UnsupportedSpecialBit,
    InvalidDate,
    InvalidPattern(String),
    IllegalCharacters(String),
    ComponentError(String), // Used for various errors specifically from `CronComponent`
    Other(String),          // Other kinds of errors
}
impl std::fmt::Display for CronError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CronError::UnsupportedSpecialBit => {
                write!(f, "CronComponent encountered an unknown special bit.")
            }
            CronError::EmptyPattern => write!(f, "CronPattern cannot be an empty string."),
            CronError::InvalidDate => write!(f, "CronScheduler encountered an invalid date."),
            CronError::InvalidPattern(msg) => write!(f, "Invalid pattern: {}", msg),
            CronError::IllegalCharacters(msg) => {
                write!(f, "Pattern contains illegal characters: {}", msg)
            }
            CronError::ComponentError(msg) => write!(f, "Component error: {}", msg),
            CronError::Other(msg) => write!(f, "An error occurred: {}", msg),
        }
    }
}
impl std::error::Error for CronError {}
