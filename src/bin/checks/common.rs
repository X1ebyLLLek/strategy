//! What several suites share: the pair of neighbours a war is fought by, a doctrine in words, how
//! far a price went back and forth, a mean and its error.

use strategy::pack::Pack;

pub use strategy::military::homefront::pick;

/// A staff's plan in words: the kinds it builds, by their shares of the army's money.
pub fn doctrine(pack: &Pack, plan: &[f32]) -> String {
    let mut order: Vec<usize> = (0..plan.len()).filter(|&u| plan[u] >= 0.05).collect();
    order.sort_by(|&a, &b| plan[b].total_cmp(&plan[a]));
    order.iter().map(|&u| format!("{} {:.0}%", pack.units[u].name, plan[u] * 100.0)).collect::<Vec<_>>().join(", ")
}

/// How far a price went back and forth over a stretch of months (log prices): the path
/// it travelled less the distance it ended up from the start, halved.
pub fn swing(log: &[f32]) -> f32 {
    let path: f32 = log.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
    let net = (log[log.len() - 1] - log[0]).abs();
    (path - net) / 2.0
}

/// Mean and its standard error.
pub fn mean_se(v: &[f32]) -> (f32, f32) {
    let n = v.len().max(1) as f32;
    let m = v.iter().sum::<f32>() / n;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / (n - 1.0).max(1.0);
    (m, (var / n).sqrt())
}
