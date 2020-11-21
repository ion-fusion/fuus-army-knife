// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::error::Error;
use toml::Value;

#[derive(Deserialize)]
pub struct FusionConfig {
    /// If true, multi-line Fusion strings (''') will have their whitespace modified
    pub format_multiline_string_contents: bool,
    /// If true, newlines won't be changed at all during the auto-format process
    /// Note: the `false` config value hasn't been implemented yet
    pub preserve_newlines: bool,
    /// Function/macro names that should have a fixed indent for their body.
    /// For example, `define`, `begin`, and `let`, may want a fixed indent to avoid crazy indentation levels.
    pub fixed_indent_symbols: Vec<String>,
    /// Function/macro names that should use fixed indent if their body is long.
    /// For example, `if` could be formatted normally if it's short, but formatted like a `define` if long.
    pub smart_indent_symbols: Vec<String>,
}

const DEFAULT_CONFIG: &'static str = r#"
[fusion]
format_multiline_string_contents = true
preserve_newlines = true
fixed_indent_symbols = [
    "lambda",
    "define",
    "begin",
    "let",
    "lets",
    "|",
]
smart_indent_symbols = [
    "if"
]
"#;

#[cfg(test)]
pub fn new_default_config() -> FusionConfig {
    DEFAULT_CONFIG
        .parse::<Value>()
        .unwrap()
        .get("fusion")
        .unwrap()
        .clone()
        .try_into::<FusionConfig>()
        .unwrap()
}

pub fn load_config(config_file_name: &str) -> Result<FusionConfig, Error> {
    let config_contents =
        std::fs::read_to_string(config_file_name).unwrap_or(DEFAULT_CONFIG.into());
    let config = config_contents.parse::<Value>().map_err(|err| {
        Error::Generic(format!(
            "Failed to parse config file: {}: {}",
            config_file_name, err
        ))
    })?;

    config
        .get("fusion")
        .ok_or_else(|| {
            Error::Generic(format!(
                "Missing config 'fusion' top-level in {}",
                config_file_name
            ))
        })?
        .clone()
        .try_into::<FusionConfig>()
        .map_err(|err| {
            Error::Generic(format!(
                "Failed to parse 'fusion' top-level config in {}: {}",
                config_file_name, err
            ))
        })
}
