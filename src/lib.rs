//! Input recognition and output formatting share a nanosecond-precision UTC value.
//! Add recognizers in `parse` and output formats in `output` independently.
pub mod output;
pub mod parse;
pub mod timezone;
