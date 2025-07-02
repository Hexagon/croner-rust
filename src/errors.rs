/// Represents errors that can occur while parsing and evaluating cron patterns.
///
/// `CronError` is used throughout the `croner` crate to indicate various types of failures
/// and is exported for consuming programs to use.
#[derive(Debug)]
pub enum CronError {
    /// The pattern string provided was empty.
    ///
    /// This error occurs if a cron pattern is set to an empty string, which is not a valid cron expression.
    EmptyPattern,

    /// Encountered an invalid date while parsing or evaluating a cron pattern.
    ///
    /// This might happen if a cron pattern results in a date that doesn't exist (e.g., February 30th).
    InvalidDate,

    /// Encountered an invalid time while parsing or evaluating a cron pattern.
    ///
    /// This can occur if a time component in the cron pattern is outside its valid range.
    InvalidTime,

    /// The search for the next valid time exceeded a reasonable limit.
    ///
    /// This is typically encountered with complex patterns that don't match any real-world times.
    TimeSearchLimitExceeded,

    /// The cron pattern provided is invalid.
    ///
    /// This error includes a message detailing the nature of the invalid pattern,
    /// such as "Pattern must consist of six fields, seconds can not be omitted."
    InvalidPattern(String),

    /// The pattern contains characters that are not allowed.
    ///
    /// This error includes a message indicating the illegal characters encountered in the pattern,
    /// such as "CronPattern contains illegal characters."
    IllegalCharacters(String),

    /// A component of the pattern is invalid.
    ///
    /// This variant is used for various errors that specifically arise from individual components of a cron pattern,
    /// such as "Position x is out of bounds for the current range (y-z).".
    ComponentError(String),
}
impl std::fmt::Display for CronError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CronError::TimeSearchLimitExceeded => {
                write!(f, "CronScheduler time search limit exceeded.")
            }
            CronError::EmptyPattern => write!(f, "CronPattern cannot be an empty string."),
            CronError::InvalidDate => write!(f, "CronScheduler encountered an invalid date."),
            CronError::InvalidTime => write!(f, "CronScheduler encountered an invalid time."),
            CronError::InvalidPattern(msg) => write!(f, "Invalid pattern: {msg}"),
            CronError::IllegalCharacters(msg) => {
                write!(f, "Pattern contains illegal characters: {msg}")
            }
            CronError::ComponentError(msg) => write!(f, "Component error: {msg}"),
        }
    }
}
impl std::error::Error for CronError {}
