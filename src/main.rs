mod annotation;
mod capture;
mod cli;
mod clipboard;
mod config;
mod geometry;
mod kdl_config;
mod overlay;
mod overlay_renderer;
mod render;
mod ring_menu;
mod save;
mod state;
mod theme;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::{
    capture::CaptureBackend,
    cli::{CaptureOptions, Cli, Command},
    clipboard::serve_png_clipboard,
    config::{config_path, AppConfig, MultiMonitorMode},
    overlay::select_region,
    save::persist_capture,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .without_time()
        .init();

    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Area(CaptureOptions::default()));

    match command {
        Command::ClipboardServe => serve_png_clipboard(),
        Command::ConfigPath => {
            println!("{}", config_path()?.display());
            Ok(())
        }
        Command::Screen(options) => {
            let config = AppConfig::load()?;
            let backend = CaptureBackend::new()?;
            let image = backend.screenshot_output(
                options.output.as_deref(),
                options.capture.show_pointer,
            )?;
            let path = if options.capture.write_to_disk {
                config.resolve_output_path(options.capture.path.as_deref())?
            } else {
                None
            };
            persist_capture(&image, path, options.capture.write_to_disk)?;
            Ok(())
        }
        Command::Area(options) => {
            let mut config = AppConfig::load()?;
            if options.unified {
                config.multi_monitor = MultiMonitorMode::Unified;
            } else if options.per_output {
                config.multi_monitor = MultiMonitorMode::PerOutput;
            }

            let backend = CaptureBackend::new()?;
            let snapshot = backend.snapshot()?;
            let selection = select_region(snapshot.clone(), config.clone(), options.show_pointer)?;

            let Some(selection) = selection else {
                return Ok(());
            };

            let image = snapshot.crop_and_stitch(
                selection.region,
                selection.show_pointer,
                &selection.annotations,
            )?;

            let write_to_disk = options.write_to_disk && selection.write_to_disk;
            let path = if write_to_disk {
                config.resolve_output_path(options.path.as_deref())?
            } else {
                None
            };
            persist_capture(&image, path, write_to_disk)?;
            Ok(())
        }
    }
}
