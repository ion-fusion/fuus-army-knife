// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::error::Error;
use std::path::PathBuf;
use toml::Value;

const NEWLINE_MODE_NO_CHANGE: &'static str = "no-change";
const NEWLINE_MODE_FIX_UP: &'static str = "fix-up";

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
    fn from_default_toml(toml: TomlFusionConfig) -> FusionConfig {
        FusionConfig {
            newline_mode: toml.newline_mode.unwrap(),
            format_multiline_string_contents: toml.format_multiline_string_contents.unwrap(),
            fixed_indent_symbols: toml.fixed_indent_symbols.unwrap(),
            smart_indent_symbols: toml.smart_indent_symbols.unwrap(),
        }
    }

    fn from_toml_with_defaults(toml: TomlFusionConfig, defaults: FusionConfig) -> FusionConfig {
        FusionConfig {
            newline_mode: toml.newline_mode.unwrap_or(defaults.newline_mode),
            format_multiline_string_contents: toml
                .format_multiline_string_contents
                .unwrap_or(defaults.format_multiline_string_contents),
            fixed_indent_symbols: toml
                .fixed_indent_symbols
                .unwrap_or(defaults.fixed_indent_symbols),
            smart_indent_symbols: toml
                .smart_indent_symbols
                .unwrap_or(defaults.smart_indent_symbols),
        }
    }
}

/// TomlFusionConfig has every member as optional so that configs can
/// be sparse and have defaults applied if values are not specified.
#[derive(Deserialize)]
struct TomlFusionConfig {
    pub newline_mode: Option<String>,
    pub format_multiline_string_contents: Option<bool>,
    pub fixed_indent_symbols: Option<Vec<String>>,
    pub smart_indent_symbols: Option<Vec<String>>,
}

impl FusionConfig {
    pub fn newline_fix_up_mode(&self) -> bool {
        self.newline_mode == NEWLINE_MODE_FIX_UP
    }
}

const DEFAULT_CONFIG: &'static str = include_str!("configs/default.toml");

pub fn new_default_config() -> FusionConfig {
    FusionConfig::from_default_toml(
        DEFAULT_CONFIG
            .parse::<Value>()
            .unwrap()
            .get("fusion")
            .unwrap()
            .clone()
            .try_into::<TomlFusionConfig>()
            .unwrap(),
    )
}

pub fn load_config(config_file_name: Option<&str>) -> Result<FusionConfig, Error> {
    let default_config = new_default_config();
    let config_path = match config_file_name {
        // Path given via CLI; just use it as is
        Some(path) => {
            let given = PathBuf::from(path);
            if !given.exists() {
                return Err(err_generic!(
                    "specified config file {:?} doesn't exist",
                    given
                ));
            }
            given
        }
        // Otherwise, look in the current working directory
        None => PathBuf::from("fuusak.toml"),
    };

    if !config_path.exists() {
        println!("Using default config...");
        return Ok(default_config);
    } else {
        println!("Using config file {:?}...", config_path);
    }

    let config_contents = std::fs::read_to_string(&config_path).map_err(|err| {
        err_generic!("Failed to read config file {:?}: {}", config_file_name, err)
    })?;
    let config = config_contents.parse::<Value>().map_err(|err| {
        err_generic!(
            "Failed to parse config file: {:?}: {}",
            config_file_name,
            err
        )
    })?;

    let config = FusionConfig::from_toml_with_defaults(
        config
            .get("fusion")
            .ok_or_else(|| {
                err_generic!(
                    "Missing config 'fusion' top-level in {:?}",
                    config_file_name
                )
            })?
            .clone()
            .try_into::<TomlFusionConfig>()
            .map_err(|err| {
                err_generic!(
                    "Failed to parse 'fusion' top-level config in {:?}: {}",
                    config_file_name,
                    err
                )
            })?,
        default_config,
    );
    if config.newline_mode != NEWLINE_MODE_NO_CHANGE && config.newline_mode != NEWLINE_MODE_FIX_UP {
        return Err(err_generic!(
            "Unknown newline mode in config: {}. Should be '{}' or '{}'",
            config.newline_mode,
            NEWLINE_MODE_NO_CHANGE,
            NEWLINE_MODE_FIX_UP
        ));
    }
    Ok(config)
}

pub fn write_default_config() -> Result<(), Error> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("fuusak.toml").map_err(|err| err_generic!("{}", err))?;
    write!(file, "{}", DEFAULT_CONFIG).map_err(|err| err_generic!("{}", err))?;
    println!("Wrote default config to fuusak.toml");
    Ok(())
}
