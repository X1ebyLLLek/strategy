//! How a side stands, as one judges its staff and generals (for the chronicles and the bot games).

use super::*;

impl Campaign {
    /// How side `s` stands, as one judges its staff and generals: the biggest stacks and the
    /// divisions in provinces that cannot feed them; divisions on the line, just behind it and deep in
    /// the rear; divisions in the provinces of its front (fewest, middle, most); pieces its land is in;
    /// the length of its front.
    pub fn stand_words(&self, pack: &Pack, s: usize) -> String {
        let c = self;
        let Some(k) = c.corps.as_ref() else { return String::new() };
        let n = c.owner.len();
        let mut count = vec![0usize; n];
        let mut eat = vec![0.0f32; n];
        for d in k.divs.iter().filter(|d| d.side == s) {
            count[d.at] += 1;
            eat[d.at] += Campaign::eats(pack, d);
        }
        let all = count.iter().sum::<usize>().max(1);
        let mut stacks: Vec<usize> = count.iter().copied().filter(|&x| x > 0).collect();
        stacks.sort_unstable_by(|a, b| b.cmp(a));
        let crowded: usize = (0..n).filter(|&p| eat[p] > c.cap_at(pack, s, p)).map(|p| count[p]).sum();
        let front = c.front(s);
        let df = c.distance_from_all(&front);
        let mut by = [0usize; 3];
        for d in k.divs.iter().filter(|d| d.side == s) {
            by[(df[d.at] as usize).min(2)] += 1;
        }
        let mut on: Vec<usize> = front.iter().map(|&p| count[p]).collect();
        on.sort_unstable();
        let mid = on.get(on.len() / 2).copied().unwrap_or(0);
        // pieces of its land (over neighbours it holds)
        let mut seen = vec![false; n];
        let mut pieces = 0;
        for p in 0..n {
            if c.owner[p] != s || seen[p] {
                continue;
            }
            pieces += 1;
            let mut q = vec![p];
            seen[p] = true;
            while let Some(x) = q.pop() {
                for &r in &c.th.nbr[x] {
                    if c.owner[r] == s && !seen[r] {
                        seen[r] = true;
                        q.push(r);
                    }
                }
            }
        }
        // (STACKS=1: who stands in the biggest stack — for finding out why it gathered)
        let mut why = String::new();
        if std::env::var("STACKS").is_ok() {
            if let Some(p) = (0..n).max_by_key(|&p| (count[p], p)) {
                let ds: Vec<&Division> = k.divs.iter().filter(|d| d.side == s && d.at == p).collect();
                let mut groups: Vec<(usize, usize)> = Vec::new();
                for d in &ds {
                    match groups.iter_mut().find(|x| x.0 == d.group) {
                        Some(x) => x.1 += 1,
                        None => groups.push((d.group, 1)),
                    }
                }
                let green = ds.iter().filter(|d| d.drill < 1.0).count();
                let manual = ds.iter().filter(|d| d.manual).count();
                let going = ds.iter().filter(|d| !d.way.is_empty()).count();
                let blow = ds.iter().filter(|d| k.blows[s].iter().any(|b| b.ids.contains(&d.id))).count();
                let rest = ds.iter().filter(|d| d.rest > 0.0).count();
                let fight = k.fights.iter().any(|f| f.at == p);
                let line = c.th.nbr[p].iter().any(|&q| c.owner[q] != s);
                let cap = p == c.th.capital[s];
                let hub = c.hubs[s].contains(&p);
                let muster = k.muster[s] == Some(p);
                why = format!(
                    "
          стопка {} в пров. {p}: группы {groups:?} | новобранцев {green} | своих приказов {manual} | в пути {going} | в ударе {blow} | отходят {rest} | бой {fight} | на линии {line} | столица {cap} | склад {hub} | сбор {muster}",
                    ds.len()
                );
            }
        }
        // recruits: all, on the line (alone there: holding a hole), going there, going back
        let mut trained_at = vec![false; n];
        for d in k.divs.iter().filter(|d| d.side == s && d.drill >= 1.0) {
            trained_at[d.at] = true;
        }
        let on_line = |p: usize| c.th.nbr[p].iter().any(|&q| c.owner[q] != s);
        let green: Vec<&Division> = k.divs.iter().filter(|d| d.side == s && d.drill < 1.0).collect();
        let g_line = green.iter().filter(|d| d.way.is_empty() && on_line(d.at)).count();
        let g_alone = green.iter().filter(|d| d.way.is_empty() && on_line(d.at) && !trained_at[d.at]).count();
        let g_to = green.iter().filter(|d| d.way.last().is_some_and(|&p| on_line(p))).count();
        let g_back = green.iter().filter(|d| d.way.last().is_some_and(|&p| !on_line(p))).count();
        let armies = c.command[s].as_ref().map_or(0, |g| g.len());
        let reserve: Vec<&Division> = k.divs.iter().filter(|d| d.side == s && d.group == NONE).collect();
        let trained = reserve.iter().filter(|d| d.drill >= 1.0).count();
        format!(
            "ИИ {s}: армий {armies}, дивизий {all}, резерв {} (обучены {trained}) | новобранцев {}: на линии {g_line} (одни {g_alone}), идут туда {g_to}, назад {g_back} | стопки {:?} | в непрокормных {:.0}% | линия/рядом/тыл {:.0}/{:.0}/{:.0}% | на пров. фронта {}/{}/{} | пустых {} из {} | кусков земли {pieces}",
            reserve.len(),
            green.len(),
            &stacks[..stacks.len().min(5)],
            100.0 * crowded as f32 / all as f32,
            100.0 * by[0] as f32 / all as f32,
            100.0 * by[1] as f32 / all as f32,
            100.0 * by[2] as f32 / all as f32,
            on.first().copied().unwrap_or(0),
            mid,
            on.last().copied().unwrap_or(0),
            on.iter().filter(|&&x| x == 0).count(),
            front.len()
        ) + &why
    }
}
