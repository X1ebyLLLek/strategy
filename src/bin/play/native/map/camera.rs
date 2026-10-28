//! The camera over the map: far away it looks straight down (the whole world as a chart); coming
//! closer it tilts towards the horizon, as in HOI4, so that the land stands up in relief. The world
//! is in world pixels (x to the east, y to the south) with the height of the land as z; the camera
//! looks north. The same numbers draw the land on the video card (`matrix`) and put the counters,
//! names and arrows over it (`to_screen`), and find what is under the mouse (`to_world`).

use eframe::egui::{pos2, Pos2, Rect};

/// The camera's angle of view, top to bottom (radians).
const FOV: f32 = 0.56;
/// The tilt close up (radians from straight down) and the zoom where it begins and is full.
const TILT: f32 = 0.80;
const TILT_FROM: f32 = 1.1;
const TILT_FULL: f32 = 7.0;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// The world point in the middle of the view, and screen points per world pixel there.
    pub cx: f32,
    pub cy: f32,
    pub k: f32,
    /// Where on the screen the map is.
    pub rect: Rect,
    /// The world's width (it goes round east to west).
    pub w: f32,
    /// The tilt (radians; 0 — straight down).
    pub pitch: f32,
}

/// How much the camera tilts at a zoom (0 far away, `TILT` close up; smooth in between).
pub fn pitch_at(k: f32) -> f32 {
    let t = ((k.ln() - TILT_FROM.ln()) / (TILT_FULL.ln() - TILT_FROM.ln())).clamp(0.0, 1.0);
    TILT * t * t * (3.0 - 2.0 * t)
}

impl Camera {
    pub fn new(cx: f32, cy: f32, k: f32, rect: Rect, w: f32, flat: bool) -> Camera {
        Camera { cx, cy, k, rect, w, pitch: if flat { 0.0 } else { pitch_at(k) } }
    }

    /// The focal length in screen points, and the camera's distance to the middle of the view.
    fn focal(&self) -> f32 {
        self.rect.height() / (2.0 * (FOV / 2.0).tan())
    }

    pub fn dist(&self) -> f32 {
        self.focal() / self.k
    }

    /// Where the camera is (world pixels, z up).
    pub fn eye(&self) -> [f32; 3] {
        self.frame().0
    }

    /// The camera's place and its axes: right (east), down the screen, forward.
    fn frame(&self) -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
        let (s, c) = self.pitch.sin_cos();
        let d = self.dist();
        let eye = [self.cx, self.cy + d * s, d * c];
        ([eye[0], eye[1], eye[2]], [1.0, 0.0, 0.0], [0.0, c, -s], [0.0, -s, -c])
    }

    /// A world point (x may be any copy of the world: the one nearest the view is taken) on the
    /// screen; and how far it is from the camera along the view.
    pub fn project(&self, x: f32, y: f32, z: f32) -> (Pos2, f32) {
        let mut x = x;
        let half = self.w / 2.0;
        while x - self.cx > half {
            x -= self.w;
        }
        while self.cx - x > half {
            x += self.w;
        }
        self.project_abs(x, y, z)
    }

    /// A world point on the screen as it is (no copy of the world chosen: for lines that must
    /// stay whole across the world's seam).
    pub fn project_abs(&self, x: f32, y: f32, z: f32) -> (Pos2, f32) {
        let (eye, r, dn, f) = self.frame();
        let v = [x - eye[0], y - eye[1], z - eye[2]];
        let dot = |a: [f32; 3]| a[0] * v[0] + a[1] * v[1] + a[2] * v[2];
        let (xv, yv, zv) = (dot(r), dot(dn), dot(f).max(1e-3));
        let fl = self.focal();
        let c = self.rect.center();
        (pos2(c.x + fl * xv / zv, c.y + fl * yv / zv), zv)
    }

    pub fn to_screen(&self, x: f32, y: f32, z: f32) -> Pos2 {
        self.project(x, y, z).0
    }

    /// How many screen points a world pixel takes at this point (smaller far up the tilted view).
    pub fn scale_at(&self, x: f32, y: f32, z: f32) -> f32 {
        self.focal() / self.project(x, y, z).1
    }

    /// The world point under a screen point: the ray from the camera through it, down to where it
    /// first meets the land (`height` gives the land's z at a world point, never above `top`):
    /// stepped down from the height of the highest peak, then halved to the point. x is the copy of
    /// the world nearest the view's middle (not wrapped).
    pub fn to_world(&self, p: Pos2, top: f32, height: &dyn Fn(f32, f32) -> f32) -> (f32, f32) {
        let (eye, r, dn, f) = self.frame();
        let fl = self.focal();
        let c = self.rect.center();
        let (a, b) = ((p.x - c.x) / fl, (p.y - c.y) / fl);
        let dir = [f[0] + a * r[0] + b * dn[0], f[1] + a * r[1] + b * dn[1], f[2] + a * r[2] + b * dn[2]];
        let dz = dir[2].min(-1e-4);
        let at = |t: f32| [eye[0] + dir[0] * t, eye[1] + dir[1] * t, eye[2] + dir[2] * t];
        // (the ray between the plane of the highest peak and a little below the sea's level: the
        // basins)
        let (t0, t1) = ((top - eye[2]) / dz, (-1.0 - eye[2]) / dz);
        if top <= 0.0 {
            let q = at(t1);
            return (q[0], q[1]);
        }
        let above = |t: f32| {
            let q = at(t);
            q[2] > height(q[0], q[1])
        };
        let steps = 64;
        let (mut lo, mut hi) = (t0, t1);
        for i in 1..=steps {
            let t = t0 + (t1 - t0) * i as f32 / steps as f32;
            if !above(t) {
                hi = t;
                break;
            }
            lo = t;
        }
        for _ in 0..16 {
            let mid = (lo + hi) / 2.0;
            if above(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let q = at(hi);
        (q[0], q[1])
    }

    /// The view's corners on the ground (z = 0): left top, right top, right bottom, left bottom.
    pub fn corners(&self) -> [(f32, f32); 4] {
        let r = self.rect;
        let flat = |_: f32, _: f32| 0.0;
        [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(|p| self.to_world(p, 0.0, &flat))
    }

    /// The world's bounds seen in the view (world pixels: x0, y0, x1, y1; x not wrapped), with a
    /// margin for the land standing up.
    pub fn seen(&self, margin: f32) -> [f32; 4] {
        let c = self.corners();
        let xs = c.map(|p| p.0);
        let ys = c.map(|p| p.1);
        let f = |v: [f32; 4], max: bool| v.iter().cloned().fold(if max { f32::MIN } else { f32::MAX }, if max { f32::max } else { f32::min });
        [f(xs, false) - margin, f(ys, false) - margin, f(xs, true) + margin, f(ys, true) + margin]
    }

    /// The matrix for the video card: world point (with the copy's shift added) to clip space of
    /// the map's rectangle, column by column (as GL reads it).
    pub fn matrix(&self, shift: f32) -> [f32; 16] {
        let (eye, r, dn, f) = self.frame();
        let fl = self.focal();
        let (hw, hh) = (self.rect.width() / 2.0, self.rect.height() / 2.0);
        let d = self.dist();
        let (near, far) = (d * 0.05, d * 8.0 + 4000.0);
        let (a, b) = ((far + near) / (far - near), -2.0 * far * near / (far - near));
        let e = [eye[0] - shift, eye[1], eye[2]];
        let dot = |u: [f32; 3]| u[0] * e[0] + u[1] * e[1] + u[2] * e[2];
        // rows: x = fl/hw · right, y = −fl/hh · down, z = a·forward + b, w = forward
        let rows = [
            [fl / hw * r[0], fl / hw * r[1], fl / hw * r[2], -fl / hw * dot(r)],
            [-fl / hh * dn[0], -fl / hh * dn[1], -fl / hh * dn[2], fl / hh * dot(dn)],
            [a * f[0], a * f[1], a * f[2], -a * dot(f) + b],
            [f[0], f[1], f[2], -dot(f)],
        ];
        let mut m = [0.0f32; 16];
        for c in 0..4 {
            for rr in 0..4 {
                m[c * 4 + rr] = rows[rr][c];
            }
        }
        m
    }
}
