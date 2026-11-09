#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use directories::ProjectDirs;
use kdl::KdlDocument;

use crate::config::MultiMonitorMode;

#[derive(Debug, Clone)]
pub struct ConchyKdlConfig {
    pub multi_monitor: MultiMonitorMode,
    pub screenshot_path: Option<String>,
    pub copy_to_clipboard_on_save: bool,
    pub ring_menu: bool,
    pub theme_name: Option<String>,
    pub default_tool: Option<String>,
    pub stroke_width: f64,
}

impl Default for ConchyKdlConfig {
    fn default() -> Self {
        Self {
            multi_monitor: MultiMonitorMode::Unified,
            screenshot_path: Some("~/Pictures/Screenshots/Screenshot_%Y-%m-%d_%H-%M-%S.png".into()),
            copy_to_clipboard_on_save: true,
            ring_menu: true,
            theme_name: Some("tokyo-night".into()),
            default_tool: Some("select".into()),
            stroke_width: 3.5,
        }
    }
}

pub fn conchy_kdl_path() -> Result<PathBuf> {
    if let Some(dirs) = ProjectDirs::from("org", "conchy-os", "conchy") {
        let p = dirs.config_dir().join("shot.kdl");
        if p.exists() {
            return Ok(p);
        }
    }
    if let Some(dirs) = ProjectDirs::from("org", "conchy-os", "conchy-shot") {
        let p = dirs.config_dir().join("config.kdl");
        if p.exists() {
            return Ok(p);
        }
    }
    // Default fallback path
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Ok(PathBuf::from(home).join(".config/conchy/shot.kdl"))
}

impl ConchyKdlConfig {
    pub fn load_or_default() -> Self {
        match conchy_kdl_path() {
            Ok(path) if path.exists() => Self::parse_file(&path).unwrap_or_default(),
            _ => Self::default(),
        }
    }

    pub fn parse_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read KDL config from {}", path.display()))?;
        let doc: KdlDocument = content
            .parse()
            .with_context(|| format!("failed to parse KDL syntax in {}", path.display()))?;

        let mut config = Self::default();

        if let Some(node) = doc.get("multi-monitor") {
            if let Some(val) = node.entries().first().and_then(|e| e.value().as_string()) {
                config.multi_monitor = match val {
                    "per-output" | "isolated" => MultiMonitorMode::PerOutput,
                    _ => MultiMonitorMode::Unified,
                };
            }
        }

        if let Some(node) = doc.get("save-path") {
            if let Some(val) = node.entries().first().and_then(|e| e.value().as_string()) {
                config.screenshot_path = Some(val.to_string());
            }
        }

        if let Some(node) = doc.get("copy-to-clipboard-on-save") {
            if let Some(val) = node.entries().first().and_then(|e| e.value().as_bool()) {
                config.copy_to_clipboard_on_save = val;
            }
        }

        if let Some(node) = doc.get("ring-menu") {
            if let Some(val) = node.entries().first().and_then(|e| e.value().as_bool()) {
                config.ring_menu = val;
            }
        }

        if let Some(ui_node) = doc.get("ui") {
            if let Some(children) = ui_node.children() {
                if let Some(n) = children.get("ring-menu") {
                    if let Some(val) = n.entries().first().and_then(|e| e.value().as_bool()) {
                        config.ring_menu = val;
                    }
                }
                if let Some(n) = children.get("theme") {
                    if let Some(val) = n.entries().first().and_then(|e| e.value().as_string()) {
                        config.theme_name = Some(val.to_string());
                    }
                }
                if let Some(n) = children.get("default-tool") {
                    if let Some(val) = n.entries().first().and_then(|e| e.value().as_string()) {
                        config.default_tool = Some(val.to_string());
                    }
                }
                if let Some(n) = children.get("stroke-width") {
                    if let Some(val) = n.entries().first().and_then(|e| e.value().as_i64()) {
                        config.stroke_width = val as f64;
                    }
                }
            }
        }

        Ok(config)
    }
}
