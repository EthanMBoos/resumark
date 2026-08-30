//! Parsing and editing for portable Resumark theme files.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

const MANIFEST_START: &str = "/* resumark-theme\n";
const MANIFEST_END: &str = "\n*/";
const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_MANIFEST_BYTES: usize = 32 * 1024;
const MAX_CONTROLS: usize = 32;

pub const THEME_FORMAT_VERSION: u32 = 1;
pub const BUNDLED_FONT_FAMILIES: &[&str] = &["Libertinus Serif", "Source Sans 3"];

/// A parsed, portable `.typ` theme and its editable manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeFile {
    source: String,
    manifest: ThemeManifest,
    manifest_range: Range<usize>,
}

/// Metadata and controls embedded at the start of a theme file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeManifest {
    pub version: u32,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub controls: Vec<ThemeControl>,
}

/// A control rendered by the web theme customizer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ThemeControl {
    Number {
        key: String,
        label: String,
        group: String,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        unit: String,
    },
    Color {
        key: String,
        label: String,
        group: String,
        value: String,
    },
    Font {
        key: String,
        label: String,
        group: String,
        value: String,
        options: Vec<String>,
    },
}

/// A new value supplied by a theme control.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeValue {
    Number(f64),
    Text(String),
}

/// A byte range in the theme source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeRange {
    pub start: usize,
    pub end: usize,
}

/// A theme-file error that can be shown beside its source.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("{message}")]
pub struct ThemeFileError {
    pub message: String,
    pub range: Option<ThemeRange>,
}

impl ThemeFile {
    /// Parses and validates a complete `.typ` theme file.
    pub fn parse(source: impl Into<String>) -> Result<Self, ThemeFileError> {
        let source = source.into();
        if source.len() > MAX_SOURCE_BYTES {
            return Err(theme_error(format!(
                "theme source is {} bytes; the limit is {MAX_SOURCE_BYTES}",
                source.len()
            )));
        }

        let content_start = source
            .strip_prefix('\u{feff}')
            .map_or(0, |_| '\u{feff}'.len_utf8());
        let leading = source[content_start..]
            .find(|character: char| !character.is_whitespace())
            .map_or(source.len(), |offset| content_start + offset);
        if !source[leading..].starts_with(MANIFEST_START) {
            return Err(ThemeFileError {
                message: "theme must start with a `resumark-theme` manifest".to_owned(),
                range: Some(ThemeRange {
                    start: leading,
                    end: leading,
                }),
            });
        }

        let json_start = leading + MANIFEST_START.len();
        let Some(relative_end) = source[json_start..].find(MANIFEST_END) else {
            return Err(ThemeFileError {
                message: "theme manifest is missing its closing `*/`".to_owned(),
                range: Some(ThemeRange {
                    start: leading,
                    end: source.len(),
                }),
            });
        };
        let json_end = json_start + relative_end;
        if json_end - json_start > MAX_MANIFEST_BYTES {
            return Err(ThemeFileError {
                message: format!("theme manifest exceeds {MAX_MANIFEST_BYTES} bytes"),
                range: Some(ThemeRange {
                    start: json_start,
                    end: json_end,
                }),
            });
        }

        let json = &source[json_start..json_end];
        let manifest = serde_json::from_str::<ThemeManifest>(json).map_err(|error| {
            let offset = json_offset(json, error.line(), error.column());
            ThemeFileError {
                message: format!("could not read theme manifest: {error}"),
                range: Some(ThemeRange {
                    start: json_start + offset,
                    end: (json_start + offset + 1).min(json_end),
                }),
            }
        })?;
        validate_manifest(&manifest, json_start..json_end)?;

        Ok(Self {
            source,
            manifest,
            manifest_range: json_start..json_end,
        })
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn manifest(&self) -> &ThemeManifest {
        &self.manifest
    }

    /// Returns the control values passed to the Typst theme.
    #[must_use]
    pub fn control_values(&self) -> Map<String, Value> {
        let mut values = Map::new();
        for control in &self.manifest.controls {
            let value = match control {
                ThemeControl::Number { value, .. } => Value::from(*value),
                ThemeControl::Color { value, .. } | ThemeControl::Font { value, .. } => {
                    Value::from(value.clone())
                }
            };
            values.insert(control.key().to_owned(), value);
        }
        values
    }

    /// Changes one manifest value and rewrites the manifest inside the source.
    pub fn set_control_value(
        &mut self,
        key: &str,
        new_value: ThemeValue,
    ) -> Result<(), ThemeFileError> {
        let control = self
            .manifest
            .controls
            .iter_mut()
            .find(|control| control.key() == key)
            .ok_or_else(|| theme_error(format!("theme has no `{key}` control")))?;

        match (control, new_value) {
            (
                ThemeControl::Number {
                    value, min, max, ..
                },
                ThemeValue::Number(next),
            ) if next.is_finite() && next >= *min && next <= *max => *value = next,
            (ThemeControl::Color { value, .. }, ThemeValue::Text(next)) if valid_color(&next) => {
                *value = next
            }
            (ThemeControl::Font { value, options, .. }, ThemeValue::Text(next))
                if options.iter().any(|option| option == &next) =>
            {
                *value = next
            }
            _ => return Err(theme_error(format!("invalid value for `{key}`"))),
        }

        let json = serde_json::to_string_pretty(&self.manifest)
            .map_err(|error| theme_error(format!("could not write theme manifest: {error}")))?;
        self.source
            .replace_range(self.manifest_range.clone(), &json);
        self.manifest_range.end = self.manifest_range.start + json.len();
        Ok(())
    }
}

impl ThemeControl {
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::Number { key, .. } | Self::Color { key, .. } | Self::Font { key, .. } => key,
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Number { label, .. } | Self::Color { label, .. } | Self::Font { label, .. } => {
                label
            }
        }
    }

    #[must_use]
    pub fn group(&self) -> &str {
        match self {
            Self::Number { group, .. } | Self::Color { group, .. } | Self::Font { group, .. } => {
                group
            }
        }
    }
}

fn validate_manifest(
    manifest: &ThemeManifest,
    manifest_range: Range<usize>,
) -> Result<(), ThemeFileError> {
    if manifest.version != THEME_FORMAT_VERSION {
        return Err(at_manifest(
            format!(
                "unsupported theme version {}; expected {THEME_FORMAT_VERSION}",
                manifest.version
            ),
            manifest_range,
        ));
    }
    if manifest.name.trim().is_empty() || manifest.name.chars().count() > 80 {
        return Err(at_manifest(
            "theme name must contain 1 to 80 characters",
            manifest_range,
        ));
    }
    if manifest.description.chars().count() > 240 {
        return Err(at_manifest(
            "theme description cannot exceed 240 characters",
            manifest_range,
        ));
    }
    if manifest.controls.len() > MAX_CONTROLS {
        return Err(at_manifest(
            format!("theme has too many controls; the limit is {MAX_CONTROLS}"),
            manifest_range,
        ));
    }

    let mut keys = BTreeSet::new();
    for control in &manifest.controls {
        let key = control.key();
        if !valid_key(key) {
            return Err(at_manifest(
                format!("`{key}` is not a valid control key"),
                manifest_range.clone(),
            ));
        }
        if !keys.insert(key) {
            return Err(at_manifest(
                format!("theme contains more than one `{key}` control"),
                manifest_range.clone(),
            ));
        }
        validate_control(control, manifest_range.clone())?;
    }
    Ok(())
}

fn validate_control(
    control: &ThemeControl,
    manifest_range: Range<usize>,
) -> Result<(), ThemeFileError> {
    if control.label().trim().is_empty() || control.group().trim().is_empty() {
        return Err(at_manifest(
            format!("`{}` must have a label and group", control.key()),
            manifest_range,
        ));
    }

    match control {
        ThemeControl::Number {
            key,
            value,
            min,
            max,
            step,
            ..
        } => {
            if ![value, min, max, step]
                .iter()
                .all(|value| value.is_finite())
                || min > max
                || value < min
                || value > max
                || *step <= 0.0
            {
                return Err(at_manifest(
                    format!("`{key}` has invalid numeric bounds or value"),
                    manifest_range,
                ));
            }
        }
        ThemeControl::Color { key, value, .. } => {
            if !valid_color(value) {
                return Err(at_manifest(
                    format!("`{key}` must use a #RRGGBB color"),
                    manifest_range,
                ));
            }
        }
        ThemeControl::Font {
            key,
            value,
            options,
            ..
        } => {
            let allowed = BUNDLED_FONT_FAMILIES
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            if options.is_empty()
                || !options
                    .iter()
                    .all(|option| allowed.contains(option.as_str()))
                || !options.iter().any(|option| option == value)
            {
                return Err(at_manifest(
                    format!("`{key}` must select from the bundled font families"),
                    manifest_range,
                ));
            }
        }
    }
    Ok(())
}

fn valid_key(key: &str) -> bool {
    let mut characters = key.chars();
    matches!(characters.next(), Some('a'..='z'))
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn valid_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn json_offset(json: &str, line: usize, column: usize) -> usize {
    let line_start = json
        .match_indices('\n')
        .nth(line.saturating_sub(2))
        .map_or(0, |(index, _)| index + 1);
    let line_text = json[line_start..].split('\n').next().unwrap_or_default();
    let column_offset = line_text
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or(line_text.len(), |(index, _)| index);
    line_start + column_offset
}

fn theme_error(message: impl Into<String>) -> ThemeFileError {
    ThemeFileError {
        message: message.into(),
        range: None,
    }
}

fn at_manifest(message: impl Into<String>, range: Range<usize>) -> ThemeFileError {
    ThemeFileError {
        message: message.into(),
        range: Some(ThemeRange {
            start: range.start,
            end: range.end,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r##"/* resumark-theme
{
  "version": 1,
  "name": "Test",
  "description": "A test theme",
  "controls": [
    {
      "kind": "color",
      "key": "accent_color",
      "label": "Accent",
      "group": "Color",
      "value": "#112233"
    }
  ]
}
*/
#let render(resume, settings, theme) = resume
"##;

    #[test]
    fn parses_a_theme_and_collects_values() {
        let theme = ThemeFile::parse(SOURCE).expect("the theme should parse");

        assert_eq!(theme.manifest.name, "Test");
        assert_eq!(theme.control_values()["accent_color"], "#112233");
    }

    #[test]
    fn changing_a_control_preserves_typst_source() {
        let mut theme = ThemeFile::parse(SOURCE).expect("the theme should parse");
        theme
            .set_control_value("accent_color", ThemeValue::Text("#abcdef".to_owned()))
            .expect("the new color should be valid");

        assert!(
            theme
                .source()
                .ends_with("#let render(resume, settings, theme) = resume\n")
        );
        assert_eq!(
            ThemeFile::parse(theme.source())
                .expect("rewritten source should parse")
                .control_values()["accent_color"],
            "#abcdef"
        );
    }

    #[test]
    fn rejects_an_invalid_manifest() {
        let error = ThemeFile::parse(SOURCE.replace("#112233", "blue"))
            .expect_err("named colors are outside the manifest format");

        assert!(error.message.contains("#RRGGBB"));
        assert!(error.range.is_some());
    }
}
