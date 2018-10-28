use codespan::{ByteIndex, ByteSpan};
use lark_debug_derive::DebugWith;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

#[cfg(test)]
use codespan::ByteOffset;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Span {
    Real(ByteSpan),
    EOF,
    Synthetic,
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Span::Real(span) => {
                let start = span.start();
                let end = span.end();

                write!(f, "{}..{}", start, end)
            }

            Span::Synthetic => write!(f, "synthetic"),
            Span::EOF => write!(f, "EOF"),
        }
    }
}

debug::debug_fallback_impl!(Span);

impl From<ByteSpan> for Span {
    fn from(v: ByteSpan) -> Self {
        Span::Real(v)
    }
}

impl Span {
    crate fn from_indices(left: ByteIndex, right: ByteIndex) -> Span {
        Span::Real(ByteSpan::new(left, right))
    }

    pub fn for_str(offset: usize, s: &str) -> Span {
        Span::from_pos(offset as u32, (offset + s.len()) as u32)
    }

    pub fn from_pos(left: u32, right: u32) -> Span {
        Span::Real(ByteSpan::new(ByteIndex(left), ByteIndex(right)))
    }

    crate fn to(&self, to: Span) -> Span {
        match (self, to) {
            (Span::Real(left), Span::Real(right)) => Span::Real(left.to(right)),
            _ => Span::Synthetic,
        }
    }

    #[cfg(test)]
    crate fn to_range(&self, start: i32) -> std::ops::Range<usize> {
        let span = match self {
            Span::Real(span) => *span,
            other => unimplemented!("Can't turn {:?} into range", other),
        };

        let start_pos = span.start() + ByteOffset(start as i64);
        let end_pos = span.end() + ByteOffset(start as i64);

        start_pos.to_usize()..end_pos.to_usize()
    }

    pub fn start(&self) -> Option<ByteIndex> {
        match self {
            Span::Real(span) => Some(span.start()),
            Span::EOF => None,
            Span::Synthetic => None,
        }
    }

    pub fn end(&self) -> Option<ByteIndex> {
        match self {
            Span::Real(span) => Some(span.end()),
            Span::EOF => None,
            Span::Synthetic => None,
        }
    }

    pub fn contains(&self, position: ByteIndex) -> bool {
        match self {
            Span::Real(span) => position >= span.start() && position < span.end(),
            Span::EOF => false,
            Span::Synthetic => false,
        }
    }
}

impl Default for Span {
    fn default() -> Span {
        Span::Synthetic
    }
}

impl Hash for Span {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Span::Synthetic => 1.hash(state),
            Span::EOF => 2.hash(state),
            Span::Real(span) => {
                3.hash(state);
                span.start().hash(state);
                span.end().hash(state);
            }
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Span::Real(span) => write!(f, "{}", span),
            Span::Synthetic => write!(f, "synthetic"),
            Span::EOF => write!(f, "end of file"),
        }
    }
}

#[derive(Copy, Clone, DebugWith, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Spanned<T>(pub T, pub Span);

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Spanned<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} at {:?}", self.0, self.1)
    }
}

impl<T> Spanned<T> {
    crate fn wrap_span(node: T, span: Span) -> Spanned<T> {
        Spanned(node, span)
    }

    crate fn from(node: T, left: ByteIndex, right: ByteIndex) -> Spanned<T> {
        Spanned(node, Span::Real(ByteSpan::new(left, right)))
    }
}

pub trait HasSpan {
    type Inner;
    fn span(&self) -> Span;
    fn node(&self) -> &Self::Inner;
    fn copy<T>(&self, other: T) -> Spanned<T> {
        Spanned::wrap_span(other, self.span())
    }
}

impl<T> HasSpan for Spanned<T> {
    type Inner = T;
    fn span(&self) -> Span {
        self.1
    }

    fn node(&self) -> &T {
        &self.0
    }
}
