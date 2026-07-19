use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalTokenMismatch {
    pub token_position: usize,
    pub expected: Option<String>,
    pub found: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseErrorSite {
    pub token_position: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    EmptyInput,
    UnknownCommand { command: String },
    MissingArgument { context: &'static str },
    InvalidValue { field: &'static str, value: String },
    UnexpectedToken { context: &'static str, token: String },
    NonCanonical { canonical: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    kind: Box<ParseErrorKind>,
    canonical_token_mismatch: Option<Box<CanonicalTokenMismatch>>,
    site: Option<Box<ParseErrorSite>>,
}

impl ParseError {
    #[must_use]
    pub fn new(kind: ParseErrorKind) -> Self {
        Self { kind: Box::new(kind), canonical_token_mismatch: None, site: None }
    }

    #[must_use]
    pub fn with_canonical_token_mismatch(mut self, mismatch: CanonicalTokenMismatch) -> Self {
        self.canonical_token_mismatch = Some(Box::new(mismatch));
        self
    }

    #[must_use]
    pub fn with_site(mut self, site: ParseErrorSite) -> Self {
        self.site = Some(Box::new(site));
        self
    }

    #[must_use]
    pub fn kind(&self) -> &ParseErrorKind {
        self.kind.as_ref()
    }

    #[must_use]
    pub fn canonical_token_mismatch(&self) -> Option<&CanonicalTokenMismatch> {
        self.canonical_token_mismatch.as_deref()
    }

    #[must_use]
    pub fn site(&self) -> Option<&ParseErrorSite> {
        self.site.as_deref()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind.as_ref() {
            ParseErrorKind::EmptyInput => f.write_str("empty command"),
            ParseErrorKind::UnknownCommand { command } => {
                write!(f, "unknown command `{command}`")
            }
            ParseErrorKind::MissingArgument { context } => write!(f, "missing argument: {context}"),
            ParseErrorKind::InvalidValue { field, value } => {
                write!(f, "invalid {field}: `{value}`")
            }
            ParseErrorKind::UnexpectedToken { context, token } => {
                write!(f, "unexpected token `{token}` while parsing {context}")
            }
            ParseErrorKind::NonCanonical { canonical } => {
                write!(f, "non-canonical command; expected `{canonical}`")?;
                if let Some(mismatch) = self.canonical_token_mismatch() {
                    write!(
                        f,
                        "; first token mismatch at position {}: expected {}, found {}",
                        mismatch.token_position,
                        describe_token(mismatch.expected.as_deref()),
                        describe_token(mismatch.found.as_deref())
                    )?;
                }
                Ok(())
            }
        }?;

        if let Some(site) = self.site() {
            write!(f, "; {}", describe_site(site))?;
        }

        Ok(())
    }
}

impl Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortabilityErrorKind {
    WhitespaceInOptionName { context: &'static str, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortabilityError {
    kind: Box<PortabilityErrorKind>,
}

impl PortabilityError {
    #[must_use]
    pub fn new(kind: PortabilityErrorKind) -> Self {
        Self { kind: Box::new(kind) }
    }

    #[must_use]
    pub fn kind(&self) -> &PortabilityErrorKind {
        self.kind.as_ref()
    }
}

impl fmt::Display for PortabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind.as_ref() {
            PortabilityErrorKind::WhitespaceInOptionName { context, name } => write!(
                f,
                "non-portable {context} option name `{name}`; GUI implementations such as ShogiHome expect a single token"
            ),
        }
    }
}

impl Error for PortabilityError {}

fn describe_token(token: Option<&str>) -> String {
    token.map_or_else(|| "end of input".to_string(), |token| format!("`{token}`"))
}

fn describe_site(site: &ParseErrorSite) -> String {
    site.token.as_deref().map_or_else(
        || {
            format!(
                "at end of input (token {}, bytes {}..{})",
                site.token_position, site.byte_start, site.byte_end
            )
        },
        |token| {
            format!(
                "at token {} (`{token}`) bytes {}..{}",
                site.token_position, site.byte_start, site.byte_end
            )
        },
    )
}
