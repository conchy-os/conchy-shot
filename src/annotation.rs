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
                number,
                color,
                radius,
            } => Annotation::Step {
                center: center.translate(dx, dy),
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
                number,
                color,
                radius,
            } => {
                cr.save()?;
                // Background circle
                cr.set_source_rgba(color[0], color[1], color[2], color[3]);
                cr.arc(center.x as f64, center.y as f64, *radius, 0.0, TAU);
                cr.fill()?;

                // Border
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
                cr.set_line_width(1.5);
                cr.arc(center.x as f64, center.y as f64, *radius, 0.0, TAU);
                cr.stroke()?;

                // Text number inside
                let text = format!("{number}");
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cr.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(radius * 1.2);
                let extents = cr.text_extents(&text)?;
                cr.move_to(
                    center.x as f64 - extents.width() / 2.0 - extents.x_bearing(),
                    center.y as f64 - extents.height() / 2.0 - extents.y_bearing(),
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

/// Applies pixelation directly to an image buffer within a bounding box.
pub fn apply_pixelate(img: &mut image::RgbaImage, rect: Rect, block_size: u32) {
    let block = block_size.max(4);
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;

    let x0 = rect.x.clamp(0, img_w) as u32;
    let y0 = rect.y.clamp(0, img_h) as u32;
    let x1 = rect.right().clamp(0, img_w) as u32;
    let y1 = rect.bottom().clamp(0, img_h) as u32;

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
