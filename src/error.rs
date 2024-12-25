use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum BdecodeError {
    #[error("Expected digit in bencoded string at position '{0}' .")]
    ExpectedDigit(usize),

    #[error("Expected colon in bencoded string at position '{0}' .")]
    ExpectedColon(usize),

    #[error("Unexpected end of file in bencoded string at position '{0}' .")]
    UnexpectedEof(usize),

    #[error("Expected value (list, dict, int or string) in bencoded string at position '{0}' .")]
    ExpectedValue(usize),

    #[error("bencoded recursion depth limit exceeded over '{0}' times.")]
    DepthExceeded(usize),

    #[error("bencoded item count limit exceeded over '{0}' .")]
    LimitExceeded(usize),

    #[error("integer overflow with string '{0}'")]
    Overflow(String),
}