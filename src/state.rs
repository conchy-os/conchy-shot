use crate::annotation::{Annotation, Tool, PALETTE};
use crate::config::MultiMonitorMode;
use crate::geometry::{Point, Rect, Size};
use crate::ring_menu::{RingAction, RingMenu};

const DEFAULT_CLICK_SIZE: i32 = 32;
const KEYBOARD_STEP: i32 = 16;

#[derive(Debug, Clone)]
pub struct OutputState {
    pub logical_rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Up,
    Down {
        output_index: usize,
        last_point: Point,
        on_capture_button: bool,
        moving: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerUpOutcome {
    None,
    Redraw,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct SelectionModel {
    outputs: Vec<OutputState>,
    selected_output: usize,
    global_rect: Rect,
    global_drag_anchor: Point,
    button: ButtonState,
    pub show_pointer: bool,
    pub mode: MultiMonitorMode,

    // Annotation state (Flameshot features)
    pub active_tool: Tool,
    pub current_color: [f64; 4],
    pub stroke_width: f64,
    pub annotations: Vec<Annotation>,
    pub in_progress_annotation: Option<Annotation>,
    pub step_counter: usize,
    pub undo_stack: Vec<Vec<Annotation>>,
    pub ring_menu: Option<RingMenu>,
    pub show_ring_menu: bool,
}

impl SelectionModel {
    pub fn new(
        outputs: Vec<OutputState>,
        default_output: usize,
        show_pointer: bool,
    ) -> Self {
        Self::with_mode(outputs, default_output, show_pointer, MultiMonitorMode::Unified, true, 3.5)
    }

    pub fn with_mode(
        outputs: Vec<OutputState>,
        default_output: usize,
        show_pointer: bool,
        mode: MultiMonitorMode,
        show_ring_menu: bool,
        stroke_width: f64,
    ) -> Self {
        let output = &outputs[default_output];
        let width = (output.logical_rect.width / 2).max(1);
        let height = (output.logical_rect.height / 2).max(1);
        let global_rect = Rect::new(
            output.logical_rect.x + output.logical_rect.width / 4,
            output.logical_rect.y + output.logical_rect.height / 4,
            width,
            height,
        );

        let mut model = Self {
            outputs,
            selected_output: default_output,
            global_rect,
            global_drag_anchor: Point::new(global_rect.x, global_rect.y),
            button: ButtonState::Up,
            show_pointer,
            mode,
            active_tool: Tool::Select,
            current_color: PALETTE[0],
            stroke_width,
            annotations: Vec::new(),
            in_progress_annotation: None,
            step_counter: 1,
            undo_stack: Vec::new(),
            ring_menu: None,
            show_ring_menu,
        };
        model.rebuild_ring_menu();
        model
    }

    pub fn selected_output_index(&self) -> usize {
        self.selected_output
    }

    pub fn global_bounding_rect(&self) -> Rect {
        if self.outputs.is_empty() {
            return Rect::new(0, 0, 1920, 1080);
        }
        let mut bounds = self.outputs[0].logical_rect;
        for out in &self.outputs[1..] {
            bounds = bounds.union(&out.logical_rect);
        }
        bounds
    }

    pub fn selection_on_output(&self, output_index: usize) -> Option<Rect> {
        let out_rect = self.outputs.get(output_index)?.logical_rect;
        self.global_rect
            .intersection(&out_rect)
            .map(|r| r.translate(-out_rect.x, -out_rect.y))
    }

    pub fn annotations_on_output(&self, output_index: usize) -> Vec<Annotation> {
        let Some(out_rect) = self.outputs.get(output_index).map(|o| o.logical_rect) else {
            return Vec::new();
        };

        let mut list: Vec<Annotation> = self
            .annotations
            .iter()
            .map(|a| a.translate(-out_rect.x, -out_rect.y))
            .collect();

        if let Some(in_progress) = &self.in_progress_annotation {
            list.push(in_progress.translate(-out_rect.x, -out_rect.y));
        }

        list
    }

    pub fn dragging_selection(&self) -> bool {
        matches!(
            self.button,
            ButtonState::Down {
                on_capture_button: false,
                ..
            }
        )
    }

    fn to_global(&self, output_index: usize, local_point: Point) -> Point {
        if let Some(out) = self.outputs.get(output_index) {
            Point::new(
                out.logical_rect.x + local_point.x,
                out.logical_rect.y + local_point.y,
            )
        } else {
            local_point
        }
    }

    pub fn rebuild_ring_menu(&mut self) {
        if !self.show_ring_menu {
            self.ring_menu = None;
            return;
        }

        if let Some(out) = self.outputs.get(self.selected_output) {
            let local_sel = self
                .global_rect
                .intersection(&out.logical_rect)
                .map(|r| r.translate(-out.logical_rect.x, -out.logical_rect.y))
                .unwrap_or_else(|| {
                    Rect::new(
                        out.logical_rect.width / 4,
                        out.logical_rect.height / 4,
                        out.logical_rect.width / 2,
                        out.logical_rect.height / 2,
                    )
                });
            let out_size = Size::new(out.logical_rect.width, out.logical_rect.height);
            self.ring_menu = Some(RingMenu::build(
                local_sel,
                out_size,
                self.active_tool,
                self.current_color,
            ));
        }
    }

    pub fn pointer_down(
        &mut self,
        output_index: usize,
        point: Point,
        over_capture_button: bool,
    ) -> bool {
        if !matches!(self.button, ButtonState::Up) {
            return false;
        }

        // Check if RingMenu was clicked
        if let Some(ring) = &self.ring_menu {
            if output_index == self.selected_output {
                if let Some(action) = ring.hit_test(point) {
                    self.handle_ring_action(action);
                    return true;
                }
            }
        }

        let global_point = self.to_global(output_index, point);
        self.selected_output = output_index;
        self.global_drag_anchor = global_point;

        if self.active_tool == Tool::Select {
            self.global_rect = Rect::new(global_point.x, global_point.y, 1, 1);
            self.button = ButtonState::Down {
                output_index,
                last_point: point,
                on_capture_button: over_capture_button,
                moving: false,
            };
        } else {
            // Start drawing annotation
            self.undo_stack.push(self.annotations.clone());
            self.button = ButtonState::Down {
                output_index,
                last_point: point,
                on_capture_button: false,
                moving: false,
            };
            self.start_annotation(global_point);
        }

        !over_capture_button
    }

    fn start_annotation(&mut self, global_point: Point) {
        match self.active_tool {
            Tool::Select => {}
            Tool::Arrow => {
                self.in_progress_annotation = Some(Annotation::Arrow {
                    start: global_point,
                    end: global_point,
                    color: self.current_color,
                    width: self.stroke_width,
                });
            }
            Tool::Rect => {
                self.in_progress_annotation = Some(Annotation::Rect {
                    rect: Rect::new(global_point.x, global_point.y, 1, 1),
                    fill: false,
                    color: self.current_color,
                    width: self.stroke_width,
                });
            }
            Tool::Pencil => {
                self.in_progress_annotation = Some(Annotation::Pencil {
                    points: vec![global_point],
                    color: self.current_color,
                    width: self.stroke_width,
                });
            }
            Tool::Blur => {
                self.in_progress_annotation = Some(Annotation::Blur {
                    rect: Rect::new(global_point.x, global_point.y, 1, 1),
                    cell_size: 10,
                });
            }
            Tool::Step => {
                let step = Annotation::Step {
                    center: global_point,
                    number: self.step_counter,
                    color: self.current_color,
                    radius: 14.0,
                };
                self.step_counter += 1;
                self.annotations.push(step);
            }
            Tool::Text => {
                let text = Annotation::Text {
                    pos: global_point,
                    text: "Note".into(),
                    color: self.current_color,
                    font_size: 16.0,
                };
                self.annotations.push(text);
            }
        }
    }

    pub fn pointer_motion(&mut self, output_index: usize, point: Point) -> bool {
        // Update RingMenu hover if on selected output
        if output_index == self.selected_output {
            if let Some(ring) = &mut self.ring_menu {
                ring.update_hover(point);
            }
        }

        let ButtonState::Down {
            on_capture_button,
            moving,
            ..
        } = self.button
        else {
            return false;
        };

        if on_capture_button {
            self.button = ButtonState::Down {
                output_index,
                last_point: point,
                on_capture_button,
                moving,
            };
            return false;
        }

        let global_point = self.to_global(output_index, point);

        if self.active_tool == Tool::Select {
            let bounds = match self.mode {
                MultiMonitorMode::Unified => self.global_bounding_rect(),
                MultiMonitorMode::PerOutput => self.outputs[self.selected_output].logical_rect,
            };

            if moving {
                let delta_x = global_point.x - self.global_drag_anchor.x;
                let delta_y = global_point.y - self.global_drag_anchor.y;
                self.global_rect = Rect::new(
                    self.global_rect.x + delta_x,
                    self.global_rect.y + delta_y,
                    self.global_rect.width,
                    self.global_rect.height,
                )
                .clamp_within(bounds);
                self.global_drag_anchor = global_point;
            } else {
                self.global_rect =
                    Rect::from_corners(self.global_drag_anchor, global_point).clamp_within(bounds);
            }
        } else {
            // Update in-progress annotation
            match &mut self.in_progress_annotation {
                Some(Annotation::Arrow { end, .. }) => {
                    *end = global_point;
                }
                Some(Annotation::Rect { rect, .. }) => {
                    *rect = Rect::from_corners(self.global_drag_anchor, global_point);
                }
                Some(Annotation::Pencil { points, .. }) => {
                    points.push(global_point);
                }
                Some(Annotation::Blur { rect, .. }) => {
                    *rect = Rect::from_corners(self.global_drag_anchor, global_point);
                }
                _ => {}
            }
        }

        self.button = ButtonState::Down {
            output_index,
            last_point: point,
            on_capture_button,
            moving,
        };
        true
    }

    pub fn pointer_up(
        &mut self,
        output_index: usize,
        point: Point,
        still_over_capture_button: bool,
    ) -> PointerUpOutcome {
        let ButtonState::Down {
            on_capture_button, ..
        } = self.button
        else {
            return PointerUpOutcome::None;
        };

        self.button = ButtonState::Up;
        if on_capture_button && still_over_capture_button {
            return PointerUpOutcome::Confirm;
        }

        if self.active_tool == Tool::Select {
            if self.global_rect.width <= 1 && self.global_rect.height <= 1 {
                let bounds = match self.mode {
                    MultiMonitorMode::Unified => self.global_bounding_rect(),
                    MultiMonitorMode::PerOutput => self.outputs[self.selected_output].logical_rect,
                };
                let global_point = self.to_global(output_index, point);
                self.global_rect = Rect::new(
                    global_point.x - DEFAULT_CLICK_SIZE / 2,
                    global_point.y - DEFAULT_CLICK_SIZE / 2,
                    DEFAULT_CLICK_SIZE,
                    DEFAULT_CLICK_SIZE,
                )
                .clamp_within(bounds);
            }
        } else {
            if let Some(ann) = self.in_progress_annotation.take() {
                self.annotations.push(ann);
            }
        }

        self.rebuild_ring_menu();
        PointerUpOutcome::Redraw
    }

    pub fn handle_ring_action(&mut self, action: RingAction) -> PointerUpOutcome {
        match action {
            RingAction::SelectTool(t) => {
                self.active_tool = t;
                self.rebuild_ring_menu();
                PointerUpOutcome::Redraw
            }
            RingAction::CycleColor => {
                self.cycle_color();
                self.rebuild_ring_menu();
                PointerUpOutcome::Redraw
            }
            RingAction::Undo => {
                self.undo();
                PointerUpOutcome::Redraw
            }
            RingAction::Copy | RingAction::Save => PointerUpOutcome::Confirm,
            RingAction::Cancel => PointerUpOutcome::Cancel,
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.annotations = prev;
            if self.step_counter > 1 {
                self.step_counter -= 1;
            }
        } else if !self.annotations.is_empty() {
            self.annotations.pop();
            if self.step_counter > 1 {
                self.step_counter -= 1;
            }
        }
    }

    pub fn cycle_color(&mut self) {
        let current_idx = PALETTE
            .iter()
            .position(|c| *c == self.current_color)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % PALETTE.len();
        self.current_color = PALETTE[next_idx];
    }

    pub fn set_move_mode(&mut self, moving: bool) {
        if let ButtonState::Down {
            output_index,
            last_point,
            on_capture_button,
            ..
        } = self.button
        {
            let global_point = self.to_global(output_index, last_point);
            self.global_drag_anchor = global_point;
            self.button = ButtonState::Down {
                output_index,
                last_point,
                on_capture_button,
                moving,
            };
        }
    }

    pub fn toggle_pointer(&mut self) {
        self.show_pointer = !self.show_pointer;
    }

    pub fn move_left(&mut self) {
        self.nudge(-KEYBOARD_STEP, 0);
    }

    pub fn move_right(&mut self) {
        self.nudge(KEYBOARD_STEP, 0);
    }

    pub fn move_up(&mut self) {
        self.nudge(0, -KEYBOARD_STEP);
    }

    pub fn move_down(&mut self) {
        self.nudge(0, KEYBOARD_STEP);
    }

    pub fn resize_left(&mut self) {
        self.resize_by(-KEYBOARD_STEP, 0);
    }

    pub fn resize_right(&mut self) {
        self.resize_by(KEYBOARD_STEP, 0);
    }

    pub fn resize_up(&mut self) {
        self.resize_by(0, -KEYBOARD_STEP);
    }

    pub fn resize_down(&mut self) {
        self.resize_by(0, KEYBOARD_STEP);
    }

    pub fn cycle_output(&mut self, delta: isize) {
        let len = self.outputs.len() as isize;
        if len <= 1 {
            return;
        }

        let new_index = (self.selected_output as isize + delta).rem_euclid(len) as usize;
        self.selected_output = new_index;
        self.rebuild_ring_menu();
    }

    pub fn capture_region(&self) -> Rect {
        self.global_rect
    }

    pub fn button_state(&self) -> ButtonState {
        self.button
    }

    fn nudge(&mut self, dx: i32, dy: i32) {
        let bounds = match self.mode {
            MultiMonitorMode::Unified => self.global_bounding_rect(),
            MultiMonitorMode::PerOutput => self.outputs[self.selected_output].logical_rect,
        };
        self.global_rect = self.global_rect.translate(dx, dy).clamp_within(bounds);
        self.rebuild_ring_menu();
    }

    fn resize_by(&mut self, dw: i32, dh: i32) {
        let bounds = match self.mode {
            MultiMonitorMode::Unified => self.global_bounding_rect(),
            MultiMonitorMode::PerOutput => self.outputs[self.selected_output].logical_rect,
        };
        self.global_rect = Rect::new(
            self.global_rect.x,
            self.global_rect.y,
            (self.global_rect.width + dw).max(1),
            (self.global_rect.height + dh).max(1),
        )
        .clamp_within(bounds);
        self.rebuild_ring_menu();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outputs() -> Vec<OutputState> {
        vec![
            OutputState {
                logical_rect: Rect::new(0, 0, 1920, 1080),
            },
            OutputState {
                logical_rect: Rect::new(1920, 0, 2560, 1440),
            },
        ]
    }

    #[test]
    fn default_rect_is_centered() {
        let model = SelectionModel::new(outputs(), 0, true);
        assert_eq!(model.capture_region(), Rect::new(480, 270, 960, 540));
    }

    #[test]
    fn cross_monitor_selection_in_unified_mode() {
        let mut model = SelectionModel::new(outputs(), 0, true);
        // Start drag on monitor 0 at (1000, 500)
        assert!(model.pointer_down(0, Point::new(1000, 500), false));
        // Drag to monitor 1 at (500, 600) -> global (2420, 600)
        assert!(model.pointer_motion(1, Point::new(500, 600)));
        assert_eq!(model.pointer_up(1, Point::new(500, 600), false), PointerUpOutcome::Redraw);

        let cap = model.capture_region();
        assert_eq!(cap.x, 1000);
        assert_eq!(cap.width, 1420); // Spans from x=1000 to x=2420 across monitor boundary!
    }

    #[test]
    fn annotation_undo_stack() {
        let mut model = SelectionModel::new(outputs(), 0, true);
        model.active_tool = Tool::Arrow;
        model.pointer_down(0, Point::new(10, 10), false);
        model.pointer_motion(0, Point::new(50, 50));
        model.pointer_up(0, Point::new(50, 50), false);
        assert_eq!(model.annotations.len(), 1);

        model.undo();
        assert_eq!(model.annotations.len(), 0);
    }
}
