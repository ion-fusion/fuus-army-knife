// Copyright Ion Fusion contributors. All Rights Reserved.

#[derive(Deserialize)]
pub struct FusionConfig {
    /// If true, multi-line Fusion strings (''') will have their whitespace modified
    pub format_multiline_string_contents: bool,
    /// If true, newlines won't be changed at all during the auto-format process
    /// Note: the `false` config value hasn't been implemented yet
    pub preserve_newlines: bool,
}

pub const DEFAULT_CONFIG: &'static str = r#"
[fusion]
format_multiline_string_contents = true
preserve_newlines = true
"#;

#[cfg(test)]
pub fn new_default_config() -> FusionConfig {
    use toml::Value;

    DEFAULT_CONFIG
        .parse::<Value>()
        .unwrap()
        .get("fusion")
        .unwrap()
        .clone()
        .try_into::<FusionConfig>()
        .unwrap()
}
