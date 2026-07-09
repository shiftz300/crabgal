// Locale / i18n — all UI-facing strings centralized here.
// Future: load from test-project/locale/zh-CN.toml or similar.
// For now, const mappings serve as both default and reserved keys.

/// Confirm dialog strings.
pub(crate) mod dialog {
    pub const QSAVE_TITLE: &str = "快速存档";
    pub const QLOAD_TITLE: &str = "快速读档";
    pub const TITLE_TITLE: &str = "返回标题画面？";
    pub const CONFIRM: &str = "确定";
    pub const CANCEL: &str = "取消";
}

/// Bottom menu button labels.
pub(crate) mod menu {
    pub const QSAVE: &str = "Q\u{00b7}SAVE";
    pub const QLOAD: &str = "Q\u{00b7}LOAD";
    pub const SAVE: &str = "SAVE";
    pub const LOAD: &str = "LOAD";
    pub const SYSTEM: &str = "SYSTEM";
    pub const TITLE: &str = "TITLE";
}
