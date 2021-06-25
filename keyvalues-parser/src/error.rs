//! All error information for parsing and rendering

use std::fmt;

use crate::text::parse::PestError;

/// Just a type alias for `Result` with a [`keyvalues::error::Error`][Error]
pub type Result<T> = std::result::Result<T, Error>;

/// All possible errors when parsing or rendering VDF text
///
/// Currently the two variants are `ParseError` which currently only occurs when `pest` encounters
/// an error in parsing text based on the grammar or `InvalidTokenStream` which all stem from
/// converting any tokenstream back to [`Vdf`][crate::Vdf]
#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub enum Error {
    #[error("Failed parsing input Error: {0}")]
    ParseError(#[from] PestError),
    #[error("Invalid token stream Context: {0}")]
    InvalidTokenStream(TokenContext),
}

impl From<TokenContext> for Error {
    fn from(context: TokenContext) -> Self {
        Self::InvalidTokenStream(context)
    }
}

/// Provides context on the specific tokenstream error
#[derive(Clone, Debug, PartialEq)]
pub enum TokenContext {
    EofWhileParsingKey,
    EofWhileParsingVal,
    EofWhileParsingSeq,
    EofWhileParsingObj,
    ExpectedSomeVal,
    ExpectedNonSeqVal,
    TrailingTokens,
}

impl fmt::Display for TokenContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EofWhileParsingKey => "Token stream ended when needed key",
            Self::EofWhileParsingVal => "Token stream ended when needed value",
            Self::EofWhileParsingSeq => "Token stream ended when parsing sequence",
            Self::EofWhileParsingObj => "Token stream ended when parsing object",
            Self::ExpectedSomeVal => "Found invalid token when expecting value",
            Self::ExpectedNonSeqVal => "Found invalid token when expecing non sequence value",
            Self::TrailingTokens => "Trailing tokens after finishing conversion",
        };

        f.write_str(message)
    }
}
