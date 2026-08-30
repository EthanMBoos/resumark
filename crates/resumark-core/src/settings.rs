use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A supported physical page size for preview and PDF output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaperSize {
    #[default]
    Letter,
    A4,
}

impl fmt::Display for PaperSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Letter => formatter.write_str("letter"),
            Self::A4 => formatter.write_str("a4"),
        }
    }
}

impl FromStr for PaperSize {
    type Err = InvalidPaperSize;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "letter" => Ok(Self::Letter),
            "a4" => Ok(Self::A4),
            _ => Err(InvalidPaperSize(value.to_owned())),
        }
    }
}

/// A paper-size name outside Resumark's supported settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPaperSize(String);

impl fmt::Display for InvalidPaperSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported paper size `{}`; use `letter` or `a4`",
            self.0
        )
    }
}

impl std::error::Error for InvalidPaperSize {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_size_names_are_case_insensitive() {
        assert_eq!("letter".parse(), Ok(PaperSize::Letter));
        assert_eq!("A4".parse(), Ok(PaperSize::A4));
    }

    #[test]
    fn unsupported_paper_size_names_are_clear() {
        let error = "legal"
            .parse::<PaperSize>()
            .expect_err("legal paper is outside the v1 policy");

        assert_eq!(
            error.to_string(),
            "unsupported paper size `legal`; use `letter` or `a4`"
        );
    }
}
