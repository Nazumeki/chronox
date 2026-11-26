//! Input recognition and output formatting share a nanosecond-precision UTC value.
//! Add recognizers in [`parse`] and output formats in [`output`] independently.
//!
//! # Examples
//!
//! ```
//! use chronox::output::{self, Format};
//! use chronox::parse;
//! use chronox::timezone::Zone;
//!
//! let zone: Zone = "Asia/Singapore".parse().unwrap();
//! let parsed = parse::parse("1_704_164_645", None, Some(zone)).unwrap();
//! assert_eq!(
//!     output::render_raw(&parsed, Some(Format::Iso8601), false, &[zone]),
//!     "2024-01-02T11:04:05+08:00\n"
//! );
//! ```
pub mod output;
pub mod parse;
pub mod style;
pub mod timezone;
