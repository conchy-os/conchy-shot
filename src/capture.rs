#![allow(dead_code)]

use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageFormat};
use libwayshot::{
    WayshotConnection,
    region::{LogicalRegion, Position, Region, Size as ShotSize},
};

use crate::geometry::Rect;

#[derive(Debug, Clone)]
pub struct CaptureOutput {
    pub name: String,
    pub logical_rect: Rect,
    pub screenshot_with_pointer: DynamicImage,
    pub screenshot_without_pointer: DynamicImage,
}

#[derive(Debug, Clone)]
pub struct CaptureSnapshot {
    pub outputs: Vec<CaptureOutput>,
}

pub struct CaptureBackend {
    conn: WayshotConnection,
}

impl CaptureBackend {
    pub fn new() -> Result<Self> {
        let conn = WayshotConnection::new().context(
            "failed to connect to the capture backend; this compositor likely does not expose wlroots screenshot protocols",
        )?;
        Ok(Self { conn })
    }

    pub fn snapshot(&self) -> Result<CaptureSnapshot> {
        let mut outputs = Vec::new();
        for output in self.conn.get_all_outputs() {
            let logical_position = output.logical_position();
            let logical_size = output.logical_size();
            let with_pointer = self
                .conn
                .screenshot_single_output(output, true)
                .with_context(|| format!("failed to capture output {}", output.name))?;
            let without_pointer = self
                .conn
                .screenshot_single_output(output, false)
                .with_context(|| format!("failed to capture output {}", output.name))?;

            outputs.push(CaptureOutput {
                name: output.name.clone(),
                logical_rect: Rect::new(
                    logical_position.x,
                    logical_position.y,
                    logical_size.width as i32,
                    logical_size.height as i32,
                ),
                screenshot_with_pointer: with_pointer,
                screenshot_without_pointer: without_pointer,
            });
        }

        if outputs.is_empty() {
            anyhow::bail!("no outputs are available for capture");
        }

        Ok(CaptureSnapshot { outputs })
    }

    pub fn screenshot_region(&self, region: Rect, show_pointer: bool) -> Result<DynamicImage> {
        self.conn
            .screenshot(region_to_logical(region), show_pointer)
            .context("failed to capture screenshot region")
    }

    pub fn screenshot_output(
        &self,
        name: Option<&str>,
        show_pointer: bool,
    ) -> Result<DynamicImage> {
        if let Some(name) = name {
            let output = self
                .conn
                .get_all_outputs()
                .iter()
                .find(|output| output.name == name)
                .with_context(|| format!("unknown output `{name}`"))?;

            return self
                .conn
                .screenshot_single_output(output, show_pointer)
                .with_context(|| format!("failed to capture output {}", output.name));
        }

        let snapshot = self.snapshot()?;
        let output = detect_output_under_pointer(&snapshot.outputs).with_context(|| {
            format!(
                "failed to determine the current monitor from the pointer; pass `--output` explicitly. Known outputs: {}",
                self.describe_outputs()
            )
        })?;

        Ok(screenshot_variant(output, show_pointer))
    }

    pub fn describe_outputs(&self) -> String {
        self.conn
            .get_all_outputs()
            .iter()
            .map(|output| {
                let position = output.logical_position();
                let size = output.logical_size();
                format!(
                    "{} @ ({}, {}) {}x{}",
                    output.name, position.x, position.y, size.width, size.height
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl CaptureSnapshot {
    pub fn crop_and_stitch(
        &self,
        region: Rect,
        show_pointer: bool,
        annotations: &[crate::annotation::Annotation],
    ) -> Result<DynamicImage> {
        let width = region.width.max(1) as u32;
        let height = region.height.max(1) as u32;
        let mut final_img = image::RgbaImage::new(width, height);

        for output in &self.outputs {
            if let Some(intersection) = region.intersection(&output.logical_rect) {
                let src_img = if show_pointer {
                    output.screenshot_with_pointer.to_rgba8()
                } else {
                    output.screenshot_without_pointer.to_rgba8()
                };

                let scale_x = src_img.width() as f64 / output.logical_rect.width.max(1) as f64;
                let scale_y = src_img.height() as f64 / output.logical_rect.height.max(1) as f64;

                let local_x = intersection.x - output.logical_rect.x;
                let local_y = intersection.y - output.logical_rect.y;

                let dst_x = (intersection.x - region.x) as u32;
                let dst_y = (intersection.y - region.y) as u32;

                for dy in 0..intersection.height as u32 {
                    for dx in 0..intersection.width as u32 {
                        let sx = ((local_x as f64 + dx as f64) * scale_x) as u32;
                        let sy = ((local_y as f64 + dy as f64) * scale_y) as u32;
                        if sx < src_img.width() && sy < src_img.height() {
                            let px = src_img.get_pixel(sx, sy);
                            let target_x = dst_x + dx;
                            let target_y = dst_y + dy;
                            if target_x < width && target_y < height {
                                final_img.put_pixel(target_x, target_y, *px);
                            }
                        }
                    }
                }
            }
        }

        // Apply blur / pixelation annotations first
        for ann in annotations {
            if let crate::annotation::Annotation::Blur { rect, cell_size } = ann {
                let local_rect = rect.translate(-region.x, -region.y);
                crate::annotation::apply_pixelate(&mut final_img, local_rect, *cell_size as u32);
            }
        }

        // Apply vector annotations via Cairo
        if !annotations.is_empty() {
            let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)?;
            {
                // Copy current pixels to cairo surface
                {
                    let mut data = surface.data()?;
                    for y in 0..height {
                        for x in 0..width {
                            let p = final_img.get_pixel(x, y);
                            let idx = ((y * width + x) * 4) as usize;
                            let a = p[3] as u32;
                            let premul = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                            data[idx] = premul(p[2]); // B
                            data[idx + 1] = premul(p[1]); // G
                            data[idx + 2] = premul(p[0]); // R
                            data[idx + 3] = p[3]; // A
                        }
                    }
                }

                let cr = cairo::Context::new(&surface)?;
                for ann in annotations {
                    if !matches!(ann, crate::annotation::Annotation::Blur { .. }) {
                        let local_ann = ann.translate(-region.x, -region.y);
                        let _ = local_ann.paint(&cr);
                    }
                }
            }

            surface.flush();
            let data = surface.data()?;
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    let b = data[idx];
                    let g = data[idx + 1];
                    let r = data[idx + 2];
                    let a = data[idx + 3];
                    let unpremul = |c: u8| {
                        if a == 0 {
                            0
                        } else {
                            ((c as u32 * 255) / a as u32).min(255) as u8
                        }
                    };
                    final_img.put_pixel(x, y, image::Rgba([unpremul(r), unpremul(g), unpremul(b), a]));
                }
            }
        }

        Ok(DynamicImage::ImageRgba8(final_img))
    }
}

pub fn encode_png(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .context("failed to encode screenshot as png")?;
    Ok(bytes)
}

fn detect_output_under_pointer(outputs: &[CaptureOutput]) -> Result<&CaptureOutput> {
    let changed_outputs = outputs
        .iter()
        .filter(|output| output_changed_when_pointer_toggled(output))
        .collect::<Vec<_>>();

    match changed_outputs.as_slice() {
        [output] => Ok(*output),
        [] => bail!(
            "could not determine which monitor contains the pointer because all output captures were identical; pass `--output` explicitly"
        ),
        outputs => {
            let names = outputs
                .iter()
                .map(|output| output.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "could not determine which monitor contains the pointer because multiple outputs changed when toggling the cursor: {names}; pass `--output` explicitly"
            )
        }
    }
}

fn output_changed_when_pointer_toggled(output: &CaptureOutput) -> bool {
    let with = output.screenshot_with_pointer.as_bytes();
    let without = output.screenshot_without_pointer.as_bytes();
    with != without
}

fn screenshot_variant(output: &CaptureOutput, show_pointer: bool) -> DynamicImage {
    if show_pointer {
        output.screenshot_with_pointer.clone()
    } else {
        output.screenshot_without_pointer.clone()
    }
}

fn region_to_logical(region: Rect) -> LogicalRegion {
    LogicalRegion {
        inner: Region {
            position: Position {
                x: region.x,
                y: region.y,
            },
            size: ShotSize {
                width: region.width as u32,
                height: region.height as u32,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixel: [u8; 4]) -> DynamicImage {
        let mut buffer = image::RgbaImage::new(1, 1);
        buffer.put_pixel(0, 0, image::Rgba(pixel));
        DynamicImage::ImageRgba8(buffer)
    }

    fn output(
        name: &str,
        with_pointer: DynamicImage,
        without_pointer: DynamicImage,
    ) -> CaptureOutput {
        CaptureOutput {
            name: name.to_owned(),
            logical_rect: Rect::new(0, 0, 100, 100),
            screenshot_with_pointer: with_pointer,
            screenshot_without_pointer: without_pointer,
        }
    }

    #[test]
    fn detects_single_output_under_pointer() {
        let unchanged = image([10, 20, 30, 255]);
        let outputs = vec![
            output("HDMI-A-1", unchanged.clone(), unchanged),
            output("DP-1", image([255, 0, 0, 255]), image([0, 0, 0, 255])),
        ];

        let selected = detect_output_under_pointer(&outputs).expect("pointer output");

        assert_eq!(selected.name, "DP-1");
    }
}
