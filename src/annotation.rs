#![allow(dead_code)]

use std::f64::consts::{PI, TAU};
use cairo::Context;
use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tool {
    #[default]
    Select,
    Arrow,
    Rect,
    Pencil,
    Blur,
    Step,
    Text,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Arrow => "Arrow",
            Tool::Rect => "Rectangle",
            Tool::Pencil => "Pencil",
            Tool::Blur => "Blur / Pixelate",
            Tool::Step => "Step Marker",
            Tool::Text => "Text",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Tool::Select => "↖",
            Tool::Arrow => "↗",
            Tool::Rect => "□",
            Tool::Pencil => "✎",
            Tool::Blur => "▦",
            Tool::Step => "①",
            Tool::Text => "T",
        }
    }
}

pub const PALETTE: [[f64; 4]; 7] = [
    [0.969, 0.463, 0.557, 1.0], // Red (#f7768e)
    [0.478, 0.635, 0.969, 1.0], // Blue (#7aa2f7)
    [0.620, 0.808, 0.416, 1.0], // Green (#9ece6a)
    [1.000, 0.620, 0.392, 1.0], // Orange (#ff9e64)
    [0.878, 0.686, 0.408, 1.0], // Yellow (#e0af68)
    [0.733, 0.604, 0.969, 1.0], // Purple (#bb9af7)
    [1.000, 1.000, 1.000, 1.0], // White (#ffffff)
];

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    Arrow {
        start: Point,
        end: Point,
        color: [f64; 4],
        width: f64,
    },
    Rect {
        rect: Rect,
        fill: bool,
        color: [f64; 4],
        width: f64,
    },
    Pencil {
        points: Vec<Point>,
        color: [f64; 4],
        width: f64,
    },
    Blur {
        rect: Rect,
        cell_size: i32,
    },
    Step {
        center: Point,
        target: Option<Point>,
        number: usize,
        color: [f64; 4],
        radius: f64,
    },
    Text {
        pos: Point,
        text: String,
        color: [f64; 4],
        font_size: f64,
    },
}

impl Annotation {
    pub fn translate(&self, dx: i32, dy: i32) -> Self {
        match self {
            Annotation::Arrow {
                start,
                end,
                color,
                width,
            } => Annotation::Arrow {
                start: start.translate(dx, dy),
                end: end.translate(dx, dy),
                color: *color,
                width: *width,
            },
            Annotation::Rect {
                rect,
                fill,
                color,
                width,
            } => Annotation::Rect {
                rect: rect.translate(dx, dy),
                fill: *fill,
                color: *color,
                width: *width,
            },
            Annotation::Pencil {
                points,
                color,
                width,
            } => Annotation::Pencil {
                points: points.iter().map(|p| p.translate(dx, dy)).collect(),
                color: *color,
                width: *width,
            },
            Annotation::Blur { rect, cell_size } => Annotation::Blur {
                rect: rect.translate(dx, dy),
                cell_size: *cell_size,
            },
            Annotation::Step {
                center,
                target,
                number,
                color,
                radius,
            } => Annotation::Step {
                center: center.translate(dx, dy),
                target: target.map(|t| t.translate(dx, dy)),
                number: *number,
                color: *color,
                radius: *radius,
            },
            Annotation::Text {
                pos,
                text,
                color,
                font_size,
            } => Annotation::Text {
                pos: pos.translate(dx, dy),
                text: text.clone(),
                color: *color,
                font_size: *font_size,
            },
        }
    }

    pub fn paint(&self, cr: &Context) -> Result<(), cairo::Error> {
        match self {
            Annotation::Arrow {
                start,
                end,
                color,
                width,
            } => {
                paint_arrow(cr, *start, *end, *color, *width)?;
            }
            Annotation::Rect {
                rect,
                fill,
                color,
                width,
            } => {
                cr.save()?;
                cr.set_source_rgba(color[0], color[1], color[2], color[3]);
                cr.rectangle(
                    rect.x as f64 + 0.5,
                    rect.y as f64 + 0.5,
                    rect.width as f64,
                    rect.height as f64,
                );
                if *fill {
                    cr.fill()?;
                } else {
                    cr.set_line_width(*width);
                    cr.stroke()?;
                }
                cr.restore()?;
            }
            Annotation::Pencil {
                points,
                color,
                width,
            } => {
                if points.len() < 2 {
                    return Ok(());
                }
                cr.save()?;
                cr.set_source_rgba(color[0], color[1], color[2], color[3]);
                cr.set_line_width(*width);
                cr.set_line_cap(cairo::LineCap::Round);
                cr.set_line_join(cairo::LineJoin::Round);
                cr.move_to(points[0].x as f64, points[0].y as f64);
                for p in &points[1..] {
                    cr.line_to(p.x as f64, p.y as f64);
                }
                cr.stroke()?;
                cr.restore()?;
            }
            Annotation::Blur { rect, .. } => {
                // Blur overlay placeholder in Cairo: a patterned translucent mosaic box
                cr.save()?;
                cr.set_source_rgba(0.1, 0.1, 0.1, 0.4);
                cr.rectangle(
                    rect.x as f64,
                    rect.y as f64,
                    rect.width as f64,
                    rect.height as f64,
                );
                cr.fill()?;
                cr.set_source_rgba(0.8, 0.8, 0.8, 0.6);
                cr.set_line_width(1.0);
                cr.set_dash(&[4.0, 4.0], 0.0);
                cr.rectangle(
                    rect.x as f64 + 0.5,
                    rect.y as f64 + 0.5,
                    rect.width as f64,
                    rect.height as f64,
                );
                cr.stroke()?;
                cr.restore()?;
            }
            Annotation::Step {
                center,
                target,
                number,
                color,
                radius,
            } => {
                cr.save()?;
                let cx = center.x as f64;
                let cy = center.y as f64;

                // Flameshot callout pin / pointer
                if let Some(target) = target {
                    let tx = target.x as f64;
                    let ty = target.y as f64;
                    let dx = tx - cx;
                    let dy = ty - cy;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist > *radius * 1.1 {
                        let nx = -dy / dist;
                        let ny = dx / dist;
                        let half_w = *radius * 0.75;
                        let p1x = cx + nx * half_w;
                        let p1y = cy + ny * half_w;
                        let p2x = cx - nx * half_w;
                        let p2y = cy - ny * half_w;

                        // Callout triangular polygon
                        cr.set_source_rgba(color[0], color[1], color[2], color[3]);
                        cr.move_to(p1x, p1y);
                        cr.line_to(tx, ty);
                        cr.line_to(p2x, p2y);
                        cr.close_path();
                        cr.fill_preserve()?;

                        // Subtle dark border on the callout pointer
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
                        cr.set_line_width(1.0);
                        cr.stroke()?;
                    }
                }

                // Background circle bubble
                cr.set_source_rgba(color[0], color[1], color[2], color[3]);
                cr.arc(cx, cy, *radius, 0.0, TAU);
                cr.fill_preserve()?;

                // Contrast border
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
                cr.set_line_width(1.5);
                cr.stroke()?;

                // Contrast text: white on dark, black on light
                let luminance = 0.299 * color[0] + 0.587 * color[1] + 0.114 * color[2];
                if luminance > 0.55 {
                    cr.set_source_rgba(0.05, 0.05, 0.08, 1.0);
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                }

                let text = format!("{number}");
                cr.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                let font_size = if *number >= 100 {
                    *radius * 0.95
                } else if *number >= 10 {
                    *radius * 1.15
                } else {
                    *radius * 1.3
                };
                cr.set_font_size(font_size);
                let extents = cr.text_extents(&text)?;
                cr.move_to(
                    cx - extents.width() / 2.0 - extents.x_bearing(),
                    cy - extents.height() / 2.0 - extents.y_bearing(),
                );
                cr.show_text(&text)?;
                cr.restore()?;
            }
            Annotation::Text {
                pos,
                text,
                color,
                font_size,
            } => {
                cr.save()?;
                cr.set_source_rgba(color[0], color[1], color[2], color[3]);
                cr.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(*font_size);
                cr.move_to(pos.x as f64, pos.y as f64);
                cr.show_text(text)?;
                cr.restore()?;
            }
        }
        Ok(())
    }
}

fn paint_arrow(
    cr: &Context,
    start: Point,
    end: Point,
    color: [f64; 4],
    width: f64,
) -> Result<(), cairo::Error> {
    let dx = (end.x - start.x) as f64;
    let dy = (end.y - start.y) as f64;
    let length = (dx * dx + dy * dy).sqrt();
    if length < 2.0 {
        return Ok(());
    }

    let angle = dy.atan2(dx);
    let head_length = (width * 3.5).clamp(14.0, 32.0).min(length * 0.45);
    let head_angle = PI / 6.0; // 30 degrees

    cr.save()?;
    cr.set_source_rgba(color[0], color[1], color[2], color[3]);
    cr.set_line_width(width);
    cr.set_line_cap(cairo::LineCap::Round);

    // Shaft line up to head
    let shaft_end_x = end.x as f64 - (head_length * 0.7) * angle.cos();
    let shaft_end_y = end.y as f64 - (head_length * 0.7) * angle.sin();

    cr.move_to(start.x as f64, start.y as f64);
    cr.line_to(shaft_end_x, shaft_end_y);
    cr.stroke()?;

    // Arrowhead (filled polygon)
    let left_x = end.x as f64 - head_length * (angle - head_angle).cos();
    let left_y = end.y as f64 - head_length * (angle - head_angle).sin();
    let right_x = end.x as f64 - head_length * (angle + head_angle).cos();
    let right_y = end.y as f64 - head_length * (angle + head_angle).sin();

    cr.move_to(end.x as f64, end.y as f64);
    cr.line_to(left_x, left_y);
    cr.line_to(shaft_end_x, shaft_end_y);
    cr.line_to(right_x, right_y);
    cr.close_path();
    cr.fill()?;

    cr.restore()?;
    Ok(())
}

/// Renders real-time mosaic pixelation of a subregion directly from the screen surface.
pub fn paint_pixelate_region(
    cr: &Context,
    bg_surface: &cairo::ImageSurface,
    rect: Rect,
    cell_size: i32,
) -> Result<(), cairo::Error> {
    let bounds = Rect::new(0, 0, bg_surface.width(), bg_surface.height());
    let Some(clipped) = rect.intersection(&bounds) else {
        return Ok(());
    };
    if clipped.width <= 1 || clipped.height <= 1 {
        return Ok(());
    }

    let block = (cell_size.max(4) as f64).clamp(6.0, 32.0);
    let rx = clipped.x as f64;
    let ry = clipped.y as f64;
    let rw = clipped.width as f64;
    let rh = clipped.height as f64;

    let small_w = ((rw / block).ceil() as i32).max(1);
    let small_h = ((rh / block).ceil() as i32).max(1);

    // Downscale the region into miniature surface
    let small_surf = cairo::ImageSurface::create(cairo::Format::ARgb32, small_w, small_h)?;
    {
        let scr = Context::new(&small_surf)?;
        scr.scale(1.0 / block, 1.0 / block);
        scr.set_source_surface(bg_surface, -rx, -ry)?;
        scr.paint()?;
    }

    cr.save()?;
    cr.rectangle(rx, ry, rw, rh);
    cr.clip();

    // Upscale with Nearest-Neighbor interpolation (pure chunky mosaic pixelation)
    let pattern = cairo::SurfacePattern::create(&small_surf);
    pattern.set_filter(cairo::Filter::Nearest);
    let matrix = cairo::Matrix::new(
        1.0 / block,
        0.0,
        0.0,
        1.0 / block,
        -rx / block,
        -ry / block,
    );
    pattern.set_matrix(matrix);

    cr.set_source(&pattern)?;
    cr.paint()?;

    // Subtle outline indicator
    cr.set_source_rgba(0.4, 0.6, 1.0, 0.4);
    cr.set_line_width(1.0);
    cr.rectangle(rx + 0.5, ry + 0.5, rw - 1.0, rh - 1.0);
    cr.stroke()?;

    cr.restore()?;
    Ok(())
}

/// Applies pixelation directly to an image buffer within a bounding box.
pub fn apply_pixelate(img: &mut image::RgbaImage, rect: Rect, block_size: u32) {
    let block = block_size.max(4);
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;

    let x0 = rect.x.clamp(0, img_w) as u32;
    let y0 = rect.y.clamp(0, img_h) as u32;
    let x1 = rect.right().clamp(0, img_w) as u32;
    let y1 = rect.bottom().clamp(0, img_h) as u32;

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for by in (y0..y1).step_by(block as usize) {
        for bx in (x0..x1).step_by(block as usize) {
            let max_x = (bx + block).min(x1);
            let max_y = (by + block).min(y1);

            let mut r_total = 0u64;
            let mut g_total = 0u64;
            let mut b_total = 0u64;
            let mut a_total = 0u64;
            let mut count = 0u64;

            for y in by..max_y {
                for x in bx..max_x {
                    let p = img.get_pixel(x, y);
                    r_total += p[0] as u64;
                    g_total += p[1] as u64;
                    b_total += p[2] as u64;
                    a_total += p[3] as u64;
                    count += 1;
                }
            }

            if count > 0 {
                let avg = image::Rgba([
                    (r_total / count) as u8,
                    (g_total / count) as u8,
                    (b_total / count) as u8,
                    (a_total / count) as u8,
                ]);

                for y in by..max_y {
                    for x in bx..max_x {
                        img.put_pixel(x, y, avg);
                    }
                }
            }
        }
    }
}
