// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::error::Error;
use toml::Value;

const NEWLINE_MODE_NO_CHANGE: &'static str = "no-change";
const NEWLINE_MODE_FIX_UP: &'static str = "fix-up";

#[derive(Deserialize)]
pub struct FusionConfig {
    /// Newline mode 'no-change' will make zero changes to newlines in the file.
    /// Mode 'fix-up' will shuffle around newlines for improved formatting.
    pub newline_mode: String,
    /// If true, multi-line Fusion strings (''') will have their whitespace modified
    pub format_multiline_string_contents: bool,
    /// Function/macro names that should have a fixed indent for their body.
    /// For example, `define`, `begin`, and `let`, may want a fixed indent to avoid crazy indentation levels.
    pub fixed_indent_symbols: Vec<String>,
    /// Function/macro names that should use fixed indent if their body is long.
    /// For example, `if` could be formatted normally if it's short, but formatted like a `define` if long.
    pub smart_indent_symbols: Vec<String>,
}

impl FusionConfig {
    pub fn newline_fix_up_mode(&self) -> bool {
        self.newline_mode == NEWLINE_MODE_FIX_UP
    }
    pub fn newline_no_change_mode(&self) -> bool {
        self.newline_mode == NEWLINE_MODE_NO_CHANGE
    }
}

const DEFAULT_CONFIG: &'static str = r#"
[fusion]
newline_mode = "fix-up"
format_multiline_string_contents = true
fixed_indent_symbols = [
    "begin",
    "define",
    "lambda",
    "let",
    "lets",
    "when",
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

    let config = config
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
        })?;
    if config.newline_mode != NEWLINE_MODE_NO_CHANGE && config.newline_mode != NEWLINE_MODE_FIX_UP {
        return Err(Error::Generic(format!(
            "Unknown newline mode in config: {}. Should be '{}' or '{}'",
            config.newline_mode, NEWLINE_MODE_NO_CHANGE, NEWLINE_MODE_FIX_UP
        )));
    }
    Ok(config)
}
