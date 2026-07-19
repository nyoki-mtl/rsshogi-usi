use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptError<E> {
    ParseFailed { line_no: usize, input: String, error: E },
    FormatMismatch { line_no: usize, input: String, expected: String, actual: String },
    UnexpectedSuccess { line_no: usize, input: String },
}

impl<E> TranscriptError<E> {
    #[must_use]
    pub const fn line_no(&self) -> usize {
        match self {
            Self::ParseFailed { line_no, .. }
            | Self::FormatMismatch { line_no, .. }
            | Self::UnexpectedSuccess { line_no, .. } => *line_no,
        }
    }
}

impl<E: fmt::Display> fmt::Display for TranscriptError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseFailed { line_no, input, error } => {
                write!(f, "line {line_no} should parse: `{input}`: {error}")
            }
            Self::FormatMismatch { line_no, input, expected, actual } => write!(
                f,
                "line {line_no} formatted mismatch for `{input}`: expected `{expected}`, got `{actual}`"
            ),
            Self::UnexpectedSuccess { line_no, input } => {
                write!(f, "line {line_no} should fail: `{input}`")
            }
        }
    }
}

impl<E> Error for TranscriptError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ParseFailed { error, .. } => Some(error),
            Self::FormatMismatch { .. } | Self::UnexpectedSuccess { .. } => None,
        }
    }
}

pub fn assert_valid_transcript<T, E, P, F>(
    content: &str,
    mut parse: P,
    mut format: F,
) -> Result<(), TranscriptError<E>>
where
    P: FnMut(&str) -> Result<T, E>,
    F: FnMut(&T) -> String,
{
    for (line_no, input, expected) in valid_cases(content) {
        let parsed = parse(input).map_err(|error| TranscriptError::ParseFailed {
            line_no,
            input: input.to_string(),
            error,
        })?;
        let actual = format(&parsed);
        if actual != expected {
            return Err(TranscriptError::FormatMismatch {
                line_no,
                input: input.to_string(),
                expected: expected.to_string(),
                actual,
            });
        }
    }

    Ok(())
}

pub fn assert_invalid_transcript<T, E, P>(
    content: &str,
    mut parse: P,
) -> Result<(), TranscriptError<E>>
where
    P: FnMut(&str) -> Result<T, E>,
{
    for (line_no, input) in invalid_cases(content) {
        if parse(input).is_ok() {
            return Err(TranscriptError::UnexpectedSuccess { line_no, input: input.to_string() });
        }
    }

    Ok(())
}

fn valid_cases(content: &str) -> impl Iterator<Item = (usize, &str, &str)> {
    content.lines().enumerate().filter_map(|(index, line)| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }

        let (input, expected) = trimmed
            .split_once("=>")
            .map_or((trimmed, trimmed), |(lhs, rhs)| (lhs.trim(), rhs.trim()));
        Some((index + 1, input, expected))
    })
}

fn invalid_cases(content: &str) -> impl Iterator<Item = (usize, &str)> {
    content.lines().enumerate().filter_map(|(index, line)| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            None
        } else {
            Some((index + 1, trimmed))
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::{format_command, parse_line, parse_line_strict};

    use super::{assert_invalid_transcript, assert_valid_transcript, TranscriptError};

    #[test]
    fn valid_transcript_helper_tracks_line_numbers() {
        let err = assert_valid_transcript(
            "usi\nposition startpos moves => position startpos",
            parse_line_strict,
            format_command,
        )
        .expect_err("second line should fail strict parsing");
        assert_eq!(
            err,
            TranscriptError::ParseFailed {
                line_no: 2,
                input: "position startpos moves".to_string(),
                error: crate::ParseError::new(crate::ParseErrorKind::NonCanonical {
                    canonical: "position startpos".to_string(),
                })
                .with_canonical_token_mismatch(crate::CanonicalTokenMismatch {
                    token_position: 3,
                    expected: None,
                    found: Some("moves".to_string()),
                })
                .with_site(crate::ParseErrorSite {
                    token_position: 3,
                    byte_start: 18,
                    byte_end: 23,
                    token: Some("moves".to_string()),
                }),
            }
        );
    }

    #[test]
    fn invalid_transcript_helper_reports_unexpected_success() {
        let err = assert_invalid_transcript("usi", parse_line)
            .expect_err("valid line should not pass invalid transcript");
        assert_eq!(
            err,
            TranscriptError::UnexpectedSuccess { line_no: 1, input: "usi".to_string() }
        );
    }
}
