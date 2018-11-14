#![feature(macro_at_most_once_rep)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(specialization)]

pub mod global;
pub mod text;

pub use self::global::{GlobalIdentifier, GlobalIdentifierTables};
pub use self::text::Text;
