//! The view: the camera fitted to what matters, held, kept on the world; points of the screen
//! and of the world, the province under the hand; the frame the land is drawn with.

use super::*;

impl MapView {
    /// The camera for the view in a rectangle of the screen (the spoiled control: never tilted).
    pub(super) fn camera(&self, rect: Rect) -> Camera {
        Camera::new(self.cx, self.cy, self.k, rect, self.ground.w as f32, crate::native::spoiled())
    }

    /// Where the view looks first: the line where the two countries meet (the front in a war),
    /// with land around it; the whole theatre if they do not meet. World pixels: x0, width, y0,
    /// height (x0 may be off the map's west: the map goes round).
    fn focus(g: &Game) -> (f32, f32, f32, f32) {
        let war = &g.h.war;
        let line: Vec<(f32, f32)> = war.front(0).into_iter().chain(war.front(1)).map(|q| Self::at(g, war.th.provs[q])).collect();
        let w = g.world.w as f32;
        let Some(&(fx, _)) = line.first() else {
            let f = g.frame;
            return (f.0 as f32, f.1 as f32, f.2 as f32, f.3 as f32);
        };
        let unwrap = |x: f32| if x - fx > w / 2.0 { x - w } else if fx - x > w / 2.0 { x + w } else { x };
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &(x, y) in &line {
            let x = unwrap(x);
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        // land on both sides of the line, and not closer than a region's size
        let pad = 40.0f32.max(0.25 * (x1 - x0).max(y1 - y0));
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let cw = (x1 - x0 + 2.0 * pad).max(360.0);
        let ch = (y1 - y0 + 2.0 * pad).max(200.0);
        (cx - cw / 2.0, cw, cy - ch / 2.0, ch)
    }

    /// The view fitted to the war, in the space the small map leaves (left of it or above it,
    /// whichever leaves it larger); the camera tilts, so the box is brought in by its corners on
    /// the screen.
    pub(super) fn fit(&mut self, g: &Game, rect: Rect) {
        let (x0, cw, y0, ch) = Self::focus(g);
        let left = Rect::from_min_max(rect.min, pos2(rect.max.x - 318.0, rect.max.y));
        let above = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - 218.0));
        let kk = |r: Rect| (r.width() / cw).min(r.height() / ch) * 0.92;
        let free = if kk(left) >= kk(above) { left } else { above };
        let (fx, fy) = (x0 + cw / 2.0, y0 + ch / 2.0);
        self.k = kk(free);
        self.cx = fx + (rect.center().x - free.center().x) / self.k;
        self.cy = fy + (rect.center().y - free.center().y) / self.k;
        let flat = |_: f32, _: f32| 0.0;
        for _ in 0..5 {
            let cam = self.camera(rect);
            let pts: Vec<Pos2> = [(x0, y0), (x0 + cw, y0), (x0 + cw, y0 + ch), (x0, y0 + ch)].iter().map(|&(x, y)| cam.project_abs(x, y, 0.0).0).collect();
            let bb = Rect::from_points(&pts);
            let s = (free.width() * 0.92 / bb.width()).min(free.height() * 0.92 / bb.height());
            self.k *= s.clamp(0.7, 1.4);
            let cam = self.camera(rect);
            let at = cam.to_world(free.center(), 0.0, &flat);
            self.cx += fx - at.0;
            self.cy += fy - at.1;
        }
        self.cx = self.cx.rem_euclid(g.world.w as f32);
    }

    /// The view held where it is put (not fitted again to the panels): for the self-checks.
    pub fn hold(&mut self) {
        self.moved = true;
        self.fitted = true;
    }

    /// The view fitted to the war again on the next frame.
    pub fn refit(&mut self) {
        self.moved = false;
        self.fitted = false;
    }

    /// The view kept on the world north and south: its top and bottom edges on the ground never
    /// past the world's (beyond them is only open sea); a view taller than the world sits in
    /// its middle. (East and west the world goes round.)
    pub(super) fn keep_on_world(&mut self, rect: Rect, h: f32) {
        self.cy = self.cy.clamp(0.0, h);
        // (the spoiled control: only the middle of the view kept on the world, as before)
        if crate::native::spoiled() {
            return;
        }
        for _ in 0..3 {
            let c = self.camera(rect).corners();
            let top = c[0].1.min(c[1].1);
            let bottom = c[2].1.max(c[3].1);
            let shift = if bottom - top >= h {
                h / 2.0 - (top + bottom) / 2.0
            } else if top < 0.0 {
                -top
            } else if bottom > h {
                h - bottom
            } else {
                0.0
            };
            if shift.abs() < 0.01 {
                break;
            }
            self.cy += shift;
        }
    }

    /// A province's point on the screen, as the map was last drawn.
    pub fn screen_of(&self, g: &Game, p: usize) -> Pos2 {
        self.pt(Self::at(g, p))
    }

    /// The view goes to the world's province `p` (as if the player moved it there).
    pub fn look_at(&mut self, g: &Game, p: usize) {
        (self.cx, self.cy) = Self::at(g, p);
        self.moved = true;
    }

    pub(super) fn at(g: &Game, p: usize) -> (f32, f32) {
        let seed = g.world.provinces[p].seed;
        ((seed % g.world.w) as f32 + 0.5, (seed / g.world.w) as f32 + 0.5)
    }

    /// The world point under a point of the screen: where the ray from the camera meets the land
    /// (the spoiled control: where it meets the sea's level, as if the land were flat).
    pub fn world_at(&self, p: Pos2) -> (f32, f32) {
        if crate::native::spoiled() {
            self.cam.to_world(p, 0.0, &|_, _| 0.0)
        } else {
            // (from the height of the highest peak down: the ridges stand above the land's own
            // heights)
            self.cam.to_world(p, self.top, &|x, y| self.z(x, y))
        }
    }

    /// The map province under a point of the screen.
    pub fn province_at(&self, g: &Game, p: Pos2) -> Option<usize> {
        let (x, y) = self.world_at(p);
        let wx = x.rem_euclid(g.world.w as f32) as usize;
        if y < 0.0 || y >= g.world.h as f32 {
            return None;
        }
        let q = g.world.owner[y as usize * g.world.w + wx.min(g.world.w - 1)];
        (q != NONE).then_some(q as usize)
    }

    /// What the video card needs for the land this frame.
    pub(super) fn frame_of(&self, cam: &Camera) -> scene::Frame {
        let w = self.ground.w as f32;
        let seen = cam.seen(RELIEF * 3.0);
        let (i0, i1) = ((seen[0] / w).floor() as i32, (seen[2] / w).floor() as i32);
        let eye = cam.eye();
        let mut copies = [(0.0f32, [0.0f32; 16], [0.0f32; 3]); 4];
        let mut n = 0;
        for i in i0..=i1.min(i0 + 3) {
            let shift = i as f32 * w;
            copies[n] = (shift, cam.matrix(shift), [eye[0] - shift, eye[1], eye[2]]);
            n += 1;
        }
        // (as HOI4: far away the countries' colours lie almost flat, close up the land shows
        // through a light tint — smoothly between)
        let near = ((self.k - 1.0) / 3.5).clamp(0.0, 1.0);
        let near = near * near * (3.0 - 2.0 * near);
        let overlay = match self.mode {
            Mode::Terrain => 0.0,
            Mode::Political => 0.74 - 0.60 * near,
            _ => 0.78,
        };
        scene::Frame {
            copies,
            n,
            k: self.k,
            time: self.born.elapsed().as_secs_f32() % 3600.0,
            overlay,
            // (the borders are drawn smooth at every zoom now — 28th session: the video card's
            // pixel borders far away gave way to the smooth ones at 1.6 with a jump)
            // (the provinces' lines by the video card: from the middle zoom, giving way close up to
            // the smooth ones — where its lines would show the world's pixels as steps)
            borders: {
                let ramp = |a: f32, b: f32| {
                    let t = ((self.k - a) / (b - a)).clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                };
                // Keep a strong, screen-space province contour at strategic zoom.  It hands off
                // to the two-layer vector contour just as that contour becomes visible.
                0.84 * ramp(0.62, 1.05) * (1.0 - ramp(1.55, 2.45))
            },
            clouds: ((0.9 - self.k) / 0.4).clamp(0.0, 1.0),
            war: false,
            relief: RELIEF,
            dist: cam.dist(),
            stripe: (6.0 / self.k).clamp(0.8, 12.0),
            era: self.era,
            hypso: if self.mode == Mode::Terrain { 1.0 } else { 0.0 },
            trees: smoothstep(3.6, 6.0, self.k),
            houses: smoothstep(2.6, 4.0, self.k),
            seen: cam.seen(RELIEF + 2.0),
            focus: [self.cx, self.cy, 1.25 * cam.dist()],
            pitch: cam.pitch,
            season: [0.0; 3],
            figures: self.figures.clone(),
        }
    }
}
