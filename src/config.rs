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
    /// Bindings that are globally available and don't need to be imported.
    /// This is used by the unbound identifier checker.
    pub global_bindings: Vec<String>,
}

impl FusionConfig {
    pub fn newline_fix_up_mode(&self) -> bool {
        self.newline_mode == NEWLINE_MODE_FIX_UP
    }
}

const DEFAULT_CONFIG: &'static str = r#"
[fusion]

# Newline mode 'no-change' will make zero changes to newlines in the file.
# Mode 'fix-up' will shuffle around newlines for improved formatting.
newline_mode = "fix-up"

# If true, multi-line Fusion strings (''') will have their whitespace modified
format_multiline_string_contents = true

# Function/macro names that should have a fixed indent for their body.
# For example, `define`, `begin`, and `let`, may want a fixed indent to avoid crazy indentation levels.
fixed_indent_symbols = [
    # Fusion defaults
    "begin",
    "cond",
    "define",
    "define_syntax",
    "if",
    "lambda",
    "let",
    "lets",
    "map",
    "unless",
    "when",
    "|",
]

# Function/macro names that should use fixed indent if their body is long.
# For example, `if` could be formatted normally if it's short, but formatted like a `define` if long.
smart_indent_symbols = [
]

# Bindings that are globally available and don't need to be imported.
# This is used by the unbound identifier checker.
global_bindings = [
    # Module "/fusion"
    "*", "+", "-", ".", "/", "<", "<=", "=", "==", "===", ">", ">=", "add",
    "adjust_day", "adjust_hour", "adjust_minute", "adjust_month", "adjust_second", "adjust_year",
    "all_defined_out", "always", "and", "annotate", "annotations", "any", "append", "apply",
    "begin", "choose", "compose", "cond", "conjoin", "curry_left", "curry_right", "decimal",
    "define", "disjoin", "display", "display_to_string", "displayln", "do", "element", "elt",
    "empty_iterator", "eof", "epoch_millis_to_timestamp", "every", "exit", "find", "first",
    "fold_left", "for", "for_fold", "for_list", "for_sexp", "for_struct", "fors", "fors_fold",
    "fors_list", "fors_sexp", "fors_struct", "has_key", "head", "ident", "identity", "if",
    "int_to_string", "ionize", "ionize_to_blob", "ionize_to_string", "is_blob", "is_bool",
    "is_clob", "is_collection", "is_decimal", "is_empty", "is_eof", "is_false", "is_float",
    "is_int", "is_iterator", "is_list", "is_null", "is_null_null", "is_pair", "is_procedure",
    "is_sequence", "is_sexp", "is_string", "is_struct", "is_symbol", "is_timestamp", "is_true",
    "is_truthy", "is_untruthy", "is_void", "iterator_append", "iterator_choose", "iterator_find",
    "iterator_has_next", "iterator_map", "iterator_map_splicing", "iterator_next", "lambda",
    "last", "let", "let_values", "letrec", "lets", "list", "list_element", "list_from_iterator",
    "list_iterator", "make_iterator", "map", "module", "negate", "none", "not", "only_in", "or",
    "pair", "prefix_in", "provide", "put", "quasiquote", "quote", "read", "remove_keys", "rename_in",
    "rename_out", "require", "retain_keys", "reverse", "same", "same_size", "set", "sexp",
    "sexp_iterator", "size", "string_append", "string_contains", "string_ends_with", "string_is_lower",
    "string_is_upper", "string_join", "string_split", "string_starts_with", "string_to_int",
    "string_to_lower", "string_to_symbol", "string_to_timestamp", "string_to_upper", "struct",
    "struct_do", "struct_for_each", "struct_iterator", "struct_merge", "struct_unzip", "struct_zip",
    "subseq", "symbol_to_string", "tail", "thunk", "timestamp", "timestamp_at_day", "timestamp_at_minute",
    "timestamp_at_month", "timestamp_at_second", "timestamp_at_year", "timestamp_day", "timestamp_hour",
    "timestamp_minute", "timestamp_month", "timestamp_now", "timestamp_offset", "timestamp_put_offset",
    "timestamp_second", "timestamp_to_epoch_millis", "timestamp_to_string", "timestamp_with_offset",
    "timestamp_year", "type_annotations", "unless", "unquote", "value_iterator", "values", "void",
    "when", "with_ion_from_file", "with_ion_from_lob", "with_ion_from_string", "write", "writeln",
    "|", "||",
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

pub fn write_default_config() -> Result<(), Error> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("fuusak.toml").map_err(|err| Error::Generic(format!("{}", err)))?;
    write!(file, "{}", DEFAULT_CONFIG).map_err(|err| Error::Generic(format!("{}", err)))?;
    Ok(())
}
