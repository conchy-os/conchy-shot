#![allow(dead_code)]

use std::f64::consts::{PI, TAU};
use cairo::Context;

use crate::annotation::Tool;
use crate::geometry::{Point, Rect, Size};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingAction {
    SelectTool(Tool),
    CycleColor,
    Undo,
    Copy,
    Save,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct RingButton {
    pub action: RingAction,
    pub icon: &'static str,
    pub tooltip: &'static str,
    pub center: Point,
    pub radius: f64,
}

#[derive(Debug, Clone)]
pub struct RingMenu {
    pub buttons: Vec<RingButton>,
    pub hovered: Option<usize>,
}

const BUTTON_RADIUS: f64 = 18.0;
const BUTTON_GAP: f64 = 42.0;

impl RingMenu {
    pub fn build(
        selection: Rect,
        output_size: Size,
        _active_tool: Tool,
        _current_color: [f64; 4],
    ) -> Self {
        let mut buttons = Vec::new();

        // 12 actions
        let actions = [
            (RingAction::SelectTool(Tool::Select), "↖", "Select/Move"),
            (RingAction::SelectTool(Tool::Arrow), "↗", "Arrow"),
            (RingAction::SelectTool(Tool::Rect), "□", "Rectangle"),
            (RingAction::SelectTool(Tool::Pencil), "✎", "Pencil"),
            (RingAction::SelectTool(Tool::Blur), "▦", "Pixelate/Blur"),
            (RingAction::SelectTool(Tool::Step), "①", "Step Marker"),
            (RingAction::SelectTool(Tool::Text), "T", "Text"),
            (RingAction::Undo, "↶", "Undo"),
            (RingAction::CycleColor, "●", "Color"),
            (RingAction::Copy, "📋", "Copy (Space)"),
            (RingAction::Save, "💾", "Save (Ctrl+S)"),
            (RingAction::Cancel, "✕", "Cancel (Esc)"),
        ];

        // Determine base position: below bottom-right of selection if space permits, else above
        let total_w = actions.len() as f64 * BUTTON_GAP;
        let mut start_x = (selection.right() as f64 - total_w).max(16.0);
        if start_x + total_w > output_size.width as f64 - 16.0 {
            start_x = (output_size.width as f64 - total_w - 16.0).max(16.0);
        }

        let mut base_y = (selection.bottom() as f64 + 28.0).min(output_size.height as f64 - 28.0);
        if base_y > output_size.height as f64 - 36.0 {
            base_y = (selection.y as f64 - 28.0).max(28.0);
        }

        for (i, (action, icon, tooltip)) in actions.iter().enumerate() {
            let cx = start_x + (i as f64) * BUTTON_GAP + BUTTON_RADIUS;
            let cy = base_y;
            buttons.push(RingButton {
                action: *action,
                icon,
                tooltip,
                center: Point::new(cx as i32, cy as i32),
                radius: BUTTON_RADIUS,
            });
        }

        Self {
            buttons,
            hovered: None,
        }
    }

    pub fn hit_test(&self, p: Point) -> Option<RingAction> {
        for btn in &self.buttons {
            let dx = (btn.center.x - p.x) as f64;
            let dy = (btn.center.y - p.y) as f64;
            if dx * dx + dy * dy <= btn.radius * btn.radius {
                return Some(btn.action);
            }
        }
        None
    }

    pub fn update_hover(&mut self, p: Point) -> bool {
        let prev = self.hovered;
        self.hovered = None;
        for (i, btn) in self.buttons.iter().enumerate() {
            let dx = (btn.center.x - p.x) as f64;
            let dy = (btn.center.y - p.y) as f64;
            if dx * dx + dy * dy <= btn.radius * btn.radius {
                self.hovered = Some(i);
                break;
            }
        }
        self.hovered != prev
    }

    pub fn paint(
        &self,
        cr: &Context,
        theme: &Theme,
        active_tool: Tool,
        current_color: [f64; 4],
    ) -> Result<(), cairo::Error> {
        if self.buttons.is_empty() {
            return Ok(());
        }

        cr.save()?;

        // Paint background dock capsule
        let first = &self.buttons[0];
        let last = &self.buttons[self.buttons.len() - 1];
        let x0 = first.center.x as f64 - first.radius - 8.0;
        let y0 = first.center.y as f64 - first.radius - 6.0;
        let w = (last.center.x - first.center.x) as f64 + (first.radius * 2.0) + 16.0;
        let h = (first.radius * 2.0) + 12.0;

        cr.set_source_rgba(0.08, 0.08, 0.12, 0.90);
        rounded_rect(cr, x0, y0, w, h, 14.0);
        cr.fill_preserve()?;
        cr.set_source_rgba(0.28, 0.30, 0.42, 0.85);
        cr.set_line_width(1.0);
        cr.stroke()?;

        // Paint individual buttons
        for (i, btn) in self.buttons.iter().enumerate() {
            let cx = btn.center.x as f64;
            let cy = btn.center.y as f64;
            let is_hovered = self.hovered == Some(i);
            let is_active_tool = match btn.action {
                RingAction::SelectTool(t) => t == active_tool,
                _ => false,
            };

            // Button circle
            if is_active_tool {
                cr.set_source_rgba(theme.accent.r, theme.accent.g, theme.accent.b, 0.9);
            } else if is_hovered {
                cr.set_source_rgba(0.25, 0.28, 0.38, 0.9);
            } else {
                cr.set_source_rgba(0.16, 0.17, 0.24, 0.85);
            }
            cr.arc(cx, cy, btn.radius, 0.0, TAU);
            cr.fill_preserve()?;

            // Button border
            if is_active_tool {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
                cr.set_line_width(1.5);
            } else {
                cr.set_source_rgba(0.35, 0.38, 0.50, 0.5);
                cr.set_line_width(1.0);
            }
            cr.stroke()?;

            // Paint clean vector icon (no emoji/font dependency)
            paint_vector_icon(cr, btn.action, cx, cy, is_active_tool, current_color)?;
        }

        cr.restore()?;
        Ok(())
    }
}

fn paint_vector_icon(
    cr: &Context,
    action: RingAction,
    cx: f64,
    cy: f64,
    is_active: bool,
    color: [f64; 4],
) -> Result<(), cairo::Error> {
    cr.save()?;
    if is_active {
        cr.set_source_rgba(0.05, 0.05, 0.08, 1.0);
    } else {
        cr.set_source_rgba(0.92, 0.94, 0.98, 1.0);
    }
    cr.set_line_width(1.6);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);

    match action {
        RingAction::SelectTool(Tool::Select) => {
            cr.move_to(cx - 5.0, cy - 7.0);
            cr.line_to(cx - 5.0, cy + 6.0);
            cr.line_to(cx - 1.5, cy + 2.5);
            cr.line_to(cx + 4.0, cy + 7.5);
            cr.line_to(cx + 6.0, cy + 5.5);
            cr.line_to(cx + 0.5, cy + 0.5);
            cr.line_to(cx + 4.5, cy - 0.5);
            cr.close_path();
            cr.fill_preserve()?;
            cr.set_line_width(1.0);
            cr.stroke()?;
        }
        RingAction::SelectTool(Tool::Arrow) => {
            cr.move_to(cx - 6.0, cy + 6.0);
            cr.line_to(cx + 5.0, cy - 5.0);
            cr.stroke()?;
            cr.move_to(cx + 5.0, cy - 1.0);
            cr.line_to(cx + 5.0, cy - 5.0);
            cr.line_to(cx + 1.0, cy - 5.0);
            cr.stroke()?;
        }
        RingAction::SelectTool(Tool::Rect) => {
            cr.rectangle(cx - 6.0, cy - 5.0, 12.0, 10.0);
            cr.stroke()?;
        }
        RingAction::SelectTool(Tool::Pencil) => {
            cr.move_to(cx - 5.0, cy + 5.0);
            cr.line_to(cx - 5.0, cy + 2.0);
            cr.line_to(cx + 2.0, cy - 5.0);
            cr.line_to(cx + 5.0, cy - 2.0);
            cr.line_to(cx - 2.0, cy + 5.0);
            cr.close_path();
            cr.stroke()?;
            cr.move_to(cx - 5.0, cy + 5.0);
            cr.line_to(cx - 4.0, cy + 2.0);
            cr.line_to(cx - 2.0, cy + 4.0);
            cr.close_path();
            cr.fill()?;
        }
        RingAction::SelectTool(Tool::Blur) => {
            let s = 3.5;
            let gap = 1.0;
            cr.rectangle(cx - s - gap, cy - s - gap, s, s);
            cr.rectangle(cx + gap, cy + gap, s, s);
            cr.fill()?;
            cr.set_line_width(1.0);
            cr.rectangle(cx + gap, cy - s - gap, s, s);
            cr.rectangle(cx - s - gap, cy + gap, s, s);
            cr.stroke()?;
        }
        RingAction::SelectTool(Tool::Step) => {
            cr.set_line_width(1.4);
            cr.arc(cx, cy, 7.5, 0.0, TAU);
            cr.stroke()?;
            cr.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(10.0);
            let ext = cr.text_extents("1")?;
            cr.move_to(
                cx - ext.width() / 2.0 - ext.x_bearing(),
                cy - ext.height() / 2.0 - ext.y_bearing(),
            );
            cr.show_text("1")?;
        }
        RingAction::SelectTool(Tool::Text) => {
            cr.move_to(cx - 5.5, cy - 6.0);
            cr.line_to(cx + 5.5, cy - 6.0);
            cr.stroke()?;
            cr.move_to(cx, cy - 6.0);
            cr.line_to(cx, cy + 6.0);
            cr.stroke()?;
        }
        RingAction::Undo => {
            cr.arc(cx + 1.0, cy + 1.0, 5.0, -PI * 0.7, PI * 0.5);
            cr.stroke()?;
            cr.move_to(cx - 5.0, cy - 1.5);
            cr.line_to(cx - 3.0, cy - 5.5);
            cr.line_to(cx - 0.5, cy - 3.5);
            cr.stroke()?;
        }
        RingAction::CycleColor => {
            cr.set_source_rgba(color[0], color[1], color[2], 1.0);
            cr.arc(cx, cy, 6.5, 0.0, TAU);
            cr.fill_preserve()?;
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
            cr.set_line_width(1.2);
            cr.stroke()?;
        }
        RingAction::Copy => {
            cr.set_line_width(1.2);
            cr.rectangle(cx - 6.0, cy - 6.0, 8.0, 9.0);
            cr.stroke()?;
            if is_active {
                cr.set_source_rgba(0.9, 0.9, 0.9, 1.0);
            } else {
                cr.set_source_rgba(0.16, 0.17, 0.24, 1.0);
            }
            cr.rectangle(cx - 2.5, cy - 2.5, 8.0, 9.0);
            cr.fill_preserve()?;
            if is_active {
                cr.set_source_rgba(0.05, 0.05, 0.08, 1.0);
            } else {
                cr.set_source_rgba(0.92, 0.94, 0.98, 1.0);
            }
            cr.stroke()?;
        }
        RingAction::Save => {
            cr.set_line_width(1.4);
            cr.move_to(cx - 6.0, cy + 2.0);
            cr.line_to(cx - 6.0, cy + 6.0);
            cr.line_to(cx + 6.0, cy + 6.0);
            cr.line_to(cx + 6.0, cy + 2.0);
            cr.stroke()?;
            cr.move_to(cx, cy - 6.0);
            cr.line_to(cx, cy + 2.5);
            cr.stroke()?;
            cr.move_to(cx - 3.5, cy - 0.5);
            cr.line_to(cx, cy + 3.0);
            cr.line_to(cx + 3.5, cy - 0.5);
            cr.stroke()?;
        }
        RingAction::Cancel => {
            cr.set_line_width(1.8);
            cr.move_to(cx - 4.5, cy - 4.5);
            cr.line_to(cx + 4.5, cy + 4.5);
            cr.stroke()?;
            cr.move_to(cx + 4.5, cy - 4.5);
            cr.line_to(cx - 4.5, cy + 4.5);
            cr.stroke()?;
        }
    }

    cr.restore()?;
    Ok(())
}

fn rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}
