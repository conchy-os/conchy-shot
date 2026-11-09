use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use smithay_client_toolkit::seat::keyboard::{Keysym, Modifiers};

use crate::kdl_config::{conchy_kdl_path, ConchyKdlConfig};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MultiMonitorMode {
    #[default]
    Unified,
    PerOutput,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub multi_monitor: MultiMonitorMode,
    pub ring_menu: bool,
    pub stroke_width: f64,
    pub screenshot_path: Option<String>,
    pub keymap: Keymap,
    pub theme: Theme,
}

#[derive(Debug, Clone)]
pub struct Keymap {
    pub confirm: Vec<KeyBinding>,
    pub copy_only: Vec<KeyBinding>,
    pub cancel: Vec<KeyBinding>,
    pub toggle_pointer: Vec<KeyBinding>,
    pub undo: Vec<KeyBinding>,
    pub move_left: Vec<KeyBinding>,
    pub move_right: Vec<KeyBinding>,
    pub move_up: Vec<KeyBinding>,
    pub move_down: Vec<KeyBinding>,
    pub resize_left: Vec<KeyBinding>,
    pub resize_right: Vec<KeyBinding>,
    pub resize_up: Vec<KeyBinding>,
    pub resize_down: Vec<KeyBinding>,
    pub next_output: Vec<KeyBinding>,
    pub previous_output: Vec<KeyBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
    pub key: Keysym,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    multi_monitor: Option<MultiMonitorMode>,
    ring_menu: Option<bool>,
    stroke_width: Option<f64>,
    screenshot_path: Option<Option<String>>,
    keymap: Option<FileKeymap>,
    theme: Option<FileThemeConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct FileThemeConfig {
    name: Option<String>,
    flavor: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileKeymap {
    confirm: Option<Vec<String>>,
    copy_only: Option<Vec<String>>,
    cancel: Option<Vec<String>>,
    toggle_pointer: Option<Vec<String>>,
    undo: Option<Vec<String>>,
    move_left: Option<Vec<String>>,
    move_right: Option<Vec<String>>,
    move_up: Option<Vec<String>>,
    move_down: Option<Vec<String>>,
    resize_left: Option<Vec<String>>,
    resize_right: Option<Vec<String>>,
    resize_up: Option<Vec<String>>,
    resize_down: Option<Vec<String>>,
    next_output: Option<Vec<String>>,
    previous_output: Option<Vec<String>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            multi_monitor: MultiMonitorMode::Unified,
            ring_menu: true,
            stroke_width: 3.5,
            screenshot_path: Some(default_screenshot_pattern()),
            keymap: Keymap::default(),
            theme: Theme::default(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // Priority 1: Check KDL config (Conchy OS standard: ~/.config/conchy/shot.kdl)
        if let Ok(kdl_path) = conchy_kdl_path() {
            if kdl_path.exists() {
                if let Ok(kdl_cfg) = ConchyKdlConfig::parse_file(&kdl_path) {
                    let theme = crate::theme::resolve_theme(
                        kdl_cfg.theme_name.as_deref(),
                        None,
                    ).unwrap_or_default();
                    return Ok(Self {
                        multi_monitor: kdl_cfg.multi_monitor,
                        ring_menu: kdl_cfg.ring_menu,
                        stroke_width: kdl_cfg.stroke_width,
                        screenshot_path: kdl_cfg.screenshot_path.or_else(|| Some(default_screenshot_pattern())),
                        keymap: Keymap::default(),
                        theme,
                    });
                }
            }
        }

        // Priority 2: Check TOML config
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let file: FileConfig = toml::from_str(&raw)
            .with_context(|| format!("failed to parse config at {}", path.display()))?;
        Self::from_file(file)
    }

    fn from_file(file: FileConfig) -> Result<Self> {
        let defaults = Keymap::default();
        let file_keymap = file.keymap.unwrap_or_default();
        let file_theme = file.theme.unwrap_or_default();
        let theme = crate::theme::resolve_theme(
            file_theme.name.as_deref(),
            file_theme.flavor.as_deref(),
        )?;

        Ok(Self {
            multi_monitor: file.multi_monitor.unwrap_or(MultiMonitorMode::Unified),
            ring_menu: file.ring_menu.unwrap_or(true),
            stroke_width: file.stroke_width.unwrap_or(3.5),
            screenshot_path: file
                .screenshot_path
                .unwrap_or_else(|| Some(default_screenshot_pattern())),
            theme,
            keymap: Keymap {
                confirm: parse_bindings(file_keymap.confirm, defaults.confirm)?,
                copy_only: parse_bindings(file_keymap.copy_only, defaults.copy_only)?,
                cancel: parse_bindings(file_keymap.cancel, defaults.cancel)?,
                toggle_pointer: parse_bindings(
                    file_keymap.toggle_pointer,
                    defaults.toggle_pointer,
                )?,
                undo: parse_bindings(file_keymap.undo, defaults.undo)?,
                move_left: parse_bindings(file_keymap.move_left, defaults.move_left)?,
                move_right: parse_bindings(file_keymap.move_right, defaults.move_right)?,
                move_up: parse_bindings(file_keymap.move_up, defaults.move_up)?,
                move_down: parse_bindings(file_keymap.move_down, defaults.move_down)?,
                resize_left: parse_bindings(file_keymap.resize_left, defaults.resize_left)?,
                resize_right: parse_bindings(file_keymap.resize_right, defaults.resize_right)?,
                resize_up: parse_bindings(file_keymap.resize_up, defaults.resize_up)?,
                resize_down: parse_bindings(file_keymap.resize_down, defaults.resize_down)?,
                next_output: parse_bindings(file_keymap.next_output, defaults.next_output)?,
                previous_output: parse_bindings(
                    file_keymap.previous_output,
                    defaults.previous_output,
                )?,
            },
        })
    }

    pub fn resolve_output_path(&self, override_path: Option<&Path>) -> Result<Option<PathBuf>> {
        if let Some(path) = override_path {
            return Ok(Some(expand_tilde(path)));
        }

        let Some(pattern) = self.screenshot_path.as_deref() else {
            return Ok(None);
        };

        let now = Local::now();
        let formatted = now.format(pattern).to_string();
        Ok(Some(expand_tilde(Path::new(&formatted))))
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            confirm: vec![binding("Return"), binding("space")],
            copy_only: vec![binding("Ctrl+c")],
            cancel: vec![binding("Escape")],
            toggle_pointer: vec![binding("p")],
            undo: vec![binding("Ctrl+z")],
            move_left: vec![binding("Left"), binding("h")],
            move_right: vec![binding("Right"), binding("l")],
            move_up: vec![binding("Up"), binding("k")],
            move_down: vec![binding("Down"), binding("j")],
            resize_left: vec![binding("Shift+Left"), binding("Shift+H")],
            resize_right: vec![binding("Shift+Right"), binding("Shift+L")],
            resize_up: vec![binding("Shift+Up"), binding("Shift+K")],
            resize_down: vec![binding("Shift+Down"), binding("Shift+J")],
            next_output: vec![binding("Tab")],
            previous_output: vec![binding("Shift+Tab")],
        }
    }
}

impl KeyBinding {
    pub fn matches(&self, keysym: Keysym, modifiers: Modifiers) -> bool {
        self.key == keysym
            && self.ctrl == modifiers.ctrl
            && self.alt == modifiers.alt
            && self.shift == modifiers.shift
            && self.logo == modifiers.logo
    }
}

pub fn config_path() -> Result<PathBuf> {
    if let Some(dirs) = ProjectDirs::from("org", "conchy-os", "conchy") {
        let p = dirs.config_dir().join("shot.toml");
        if p.exists() {
            return Ok(p);
        }
    }
    let dirs = ProjectDirs::from("org", "conchy-os", "conchy-shot")
        .context("could not resolve XDG project directories")?;
    Ok(dirs.config_dir().join("config.toml"))
}

pub fn default_screenshot_pattern() -> String {
    let base = UserDirs::new()
        .and_then(|dirs| dirs.picture_dir().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("~/Pictures"));
    base.join("Screenshots")
        .join("Screenshot_%Y-%m-%d_%H-%M-%S.png")
        .to_string_lossy()
        .into_owned()
}

fn expand_tilde(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(user_dirs) = UserDirs::new() {
            return user_dirs.home_dir().join(stripped);
        }
    }
    path.to_path_buf()
}

fn parse_bindings(
    raw: Option<Vec<String>>,
    default: Vec<KeyBinding>,
) -> Result<Vec<KeyBinding>> {
    let Some(raw) = raw else {
        return Ok(default);
    };

    raw.into_iter().map(|entry| parse_binding(&entry)).collect()
}

fn parse_binding(raw: &str) -> Result<KeyBinding> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut logo = false;
    let mut key = None;

    for part in raw.split('+') {
        let trimmed = part.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            "super" | "logo" | "mod4" => logo = true,
            _ => {
                if key.replace(parse_keysym(trimmed)?).is_some() {
                    anyhow::bail!("multiple non-modifier keys in binding: {raw}");
                }
            }
        }
    }

    Ok(KeyBinding {
        ctrl,
        alt,
        shift,
        logo,
        key: key.context("binding is missing a key")?,
    })
}

fn parse_keysym(raw: &str) -> Result<Keysym> {
    let lower = raw.trim().to_ascii_lowercase();
    let key = match lower.as_str() {
        "escape" | "esc" => Keysym::Escape,
        "return" | "enter" => Keysym::Return,
        "space" => Keysym::space,
        "tab" => Keysym::Tab,
        "left" => Keysym::Left,
        "right" => Keysym::Right,
        "up" => Keysym::Up,
        "down" => Keysym::Down,
        "h" => Keysym::h,
        "j" => Keysym::j,
        "k" => Keysym::k,
        "l" => Keysym::l,
        "a" => Keysym::a,
        "r" => Keysym::r,
        "p" => Keysym::p,
        "b" => Keysym::b,
        "s" => Keysym::s,
        "t" => Keysym::t,
        "v" => Keysym::v,
        "c" => Keysym::c,
        "z" => Keysym::z,
        _ => anyhow::bail!("unsupported keysym `{raw}`"),
    };
    Ok(key)
}

fn binding(raw: &str) -> KeyBinding {
    parse_binding(raw).expect("valid default key binding")
}
