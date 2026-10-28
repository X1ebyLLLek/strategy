//! Building meshes on the processor: a model is a list of vertices of eight numbers each —
//! position, normal, which part of the model (its colour), how much light reaches it.

pub type Model = Vec<f32>;

/// A vertex (the normal made unit).
pub fn push(m: &mut Model, p: [f32; 3], n: [f32; 3], part: f32, ao: f32) {
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    m.extend_from_slice(&[p[0], p[1], p[2], n[0] / l, n[1] / l, n[2] / l, part, ao]);
}

pub fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}
