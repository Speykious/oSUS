use miette::{Diagnostic, NamedSource, SourceOffset, SourceSpan};
use nom::error::{ContextError, ParseError};
use nom::Offset;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
pub struct BeatmapError {
    /// Source string for the beatmap file that failed to parse.
    #[source_code]
    pub input: NamedSource,

    /// Offset in chars of the error.
    #[label("{}", label.unwrap_or("here"))]
    pub span: SourceSpan,

    /// Label text for this span. Defaults to `"here"`.
    pub label: Option<&'static str>,

    /// Suggestion for fixing the parser error.
    #[help]
    pub help: Option<&'static str>,

    /// Specific error kind for this parser error.
    pub kind: BeatmapErrorKind,
}

impl BeatmapError {
    pub fn from_source_and_parse_error(
        name: impl AsRef<str>,
        source: &'static str,
        error: BeatmapParseError<&str>,
    ) -> Self {
        BeatmapError {
            input: NamedSource::new(name, source),
            span: SourceSpan::new(
                SourceOffset::from(source.offset(error.input)),
                SourceOffset::from(error.len),
            ),
            label: error.label,
            help: error.help,
            kind: if let Some(context) = error.context {
                BeatmapErrorKind::Context(context)
            } else {
                error.kind.unwrap_or(BeatmapErrorKind::Other)
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error, Diagnostic)]
pub enum BeatmapErrorKind {
    #[error("Unknown section {0:?}")]
    #[diagnostic(code(osu::unknown_section))]
    UnknownSection(String),

    /// Generic parsing error. The given context string denotes the component
    /// that failed to parse.
    #[error("Expected {0}.")]
    #[diagnostic(code(osu::parse_component))]
    Context(&'static str),

    /// Generic unspecified error. If this is returned, the call site should
    /// be annotated with context, if possible.
    #[error("An unspecified error occurred")]
    #[diagnostic(code(osu::other))]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatmapParseError<I> {
    pub input: I,
    pub len: usize,
    pub context: Option<&'static str>,
    pub label: Option<&'static str>,
    pub help: Option<&'static str>,
    pub kind: Option<BeatmapErrorKind>,
    pub touched: bool,
}

impl<I> ParseError<I> for BeatmapParseError<I> {
    fn from_error_kind(input: I, _kind: nom::error::ErrorKind) -> Self {
        Self {
            input,
            len: 0,
            label: None,
            help: None,
            context: None,
            kind: None,
            touched: false,
        }
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<I> ContextError<I> for BeatmapParseError<I> {
    fn add_context(_input: I, ctx: &'static str, mut other: Self) -> Self {
        other.context = other.context.or(Some(ctx));
        other
    }
}
