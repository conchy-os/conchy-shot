#![allow(dead_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn translate(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn distance_squared(self, other: Point) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        dx * dx + dy * dy
    }

    pub fn distance(self, other: Point) -> f64 {
        self.distance_squared(other).sqrt()
    }
}

impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_corners(a: Point, b: Point) -> Self {
        let x1 = a.x.min(b.x);
        let y1 = a.y.min(b.y);
        let x2 = a.x.max(b.x);
        let y2 = a.y.max(b.y);
        Self {
            x: x1,
            y: y1,
            width: (x2 - x1).max(1),
            height: (y2 - y1).max(1),
        }
    }

    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        if x2 > x1 && y2 > y1 {
            Some(Rect {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            })
        } else {
            None
        }
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self.right().max(other.right());
        let y2 = self.bottom().max(other.bottom());
        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }

    pub fn translate(&self, dx: i32, dy: i32) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            width: self.width,
            height: self.height,
        }
    }

    pub fn clamp_within(self, bounds: Rect) -> Self {
        let width = self.width.clamp(1, bounds.width.max(1));
        let height = self.height.clamp(1, bounds.height.max(1));
        let x = self.x.clamp(bounds.x, bounds.x + bounds.width - width);
        let y = self.y.clamp(bounds.y, bounds.y + bounds.height - height);
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}) {}x{}", self.x, self.y, self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0, 0, 1920, 1080);
        let r2 = Rect::new(1000, 500, 1500, 800);
        let inter = r1.intersection(&r2).unwrap();
        assert_eq!(inter, Rect::new(1000, 500, 920, 580));
    }

    #[test]
    fn test_rect_union() {
        let r1 = Rect::new(0, 0, 1920, 1080);
        let r2 = Rect::new(1920, 0, 2560, 1440);
        let u = r1.union(&r2);
        assert_eq!(u, Rect::new(0, 0, 4480, 1440));
    }
}
