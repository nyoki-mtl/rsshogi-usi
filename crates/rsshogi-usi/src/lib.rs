//! Engine-agnostic USI protocol surface shared across rshogi engines.

mod error;
mod format;
mod parser;
mod transcript;

pub use error::{
    CanonicalTokenMismatch, ParseError, ParseErrorKind, ParseErrorSite, PortabilityError,
    PortabilityErrorKind,
};
pub use format::format_command;
pub use parser::{
    parse_line, parse_line_strict, validate_portable_command, BestMove, BestMoveKind,
    CheckmateResponse, GameResult, GoMate, GoParams, InfoCommand, InfoScore, MateScore,
    PositionReplayError, PositionSpec, ScoreBound, ScoreValue, UsiCommand, UsiCommandDirection,
    UsiOption, UsiOptionKind,
};
pub use transcript::{assert_invalid_transcript, assert_valid_transcript, TranscriptError};
