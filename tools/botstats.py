"""What the games of bots came to (`tools/botgame.py` writes them): a line per game, then by era.

    python tools/botstats.py [OUT]      (OUT = out/botgames; the table also goes to OUT/stats.txt)
"""
import csv
import glob
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def num(x):
    try:
        return float(x)
    except ValueError:
        return 0.0


def game(path):
    chron = open(os.path.join(path, "chronicle.txt"), encoding="utf-8").read()
    m = re.search(r"карта (\d+), эпоха «(.+?)»", chron)
    if not m or "════ ИТОГ ════" not in chron:
        return None
    g = {"dir": os.path.basename(path), "seed": m.group(1), "era": m.group(2)}
    names = re.findall(r"^(\S[^:\n]*): людей ([\d.]+) млн, выпуск ([\d.]+) в мес", chron, re.M)[:2]
    g["names"] = [n[0] for n in names]
    rows = list(csv.DictReader(open(os.path.join(path, "months.csv"), encoding="utf-8")))
    war = [r for r in rows if r["war"] == "1"]
    pre = [r for r in rows if r["war"] == "0"]
    last_peace = pre[-1] if pre else rows[0]
    g["gdp"] = [num(last_peace["a_gdp"]), num(last_peace["b_gdp"])]
    g["people"] = [num(last_peace["a_people"]), num(last_peace["b_people"])]
    g["units"] = [num(last_peace["a_units"]), num(last_peace["b_units"])]
    g["anger_peace"] = [num(last_peace["a_anger"]), num(last_peace["b_anger"])]
    g["anger_start"] = [num(rows[0]["a_anger"]), num(rows[0]["b_anger"])]
    g["gdp_peace"] = [num(last_peace["a_gdp_rel"]), num(last_peace["b_gdp_rel"])]
    end = rows[-1]
    g["war_months"] = len(war)
    g["prov"] = [int(num(end["a_provinces"])), int(num(end["b_provinces"]))]
    g["prov0"] = [int(num(war[0]["a_provinces"])), int(num(war[0]["b_provinces"]))] if war else g["prov"]
    g["dead"] = [num(end["a_dead_k"]), num(end["b_dead_k"])]
    g["anger_end"] = [num(end["a_anger"]), num(end["b_anger"])]
    fell = re.search(r"^(.+?) капитулировала на (\d+)-м месяце войны", chron, re.M)
    g["fell"] = g["names"].index(fell.group(1)) if fell and fell.group(1) in g["names"] else None
    g["battles"] = list(csv.DictReader(open(os.path.join(path, "battles.csv"), encoding="utf-8")))
    g["designs"] = len(re.findall(r" — разработка: ", chron))
    g["unrest"] = len(re.findall(r"^• .*(волнения|стачк|бунт|восстан)", chron, re.M))
    g["generals"] = []
    tail = chron.split("════ ИТОГ ════")[1]
    for m in re.finditer(r"группа: (.+?) \(образование ([\d.]+), опыт ([\d.]+), талант ([\d.]+), смелость ([+-][\d.]+)\): побед (\d+), поражений (\d+), взял провинций (\d+)", tail):
        g["generals"].append(dict(name=m.group(1), edu=float(m.group(2)), talent=float(m.group(4)), bold=float(m.group(5)), won=int(m.group(6)), lost=int(m.group(7)), took=int(m.group(8))))
    return g


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "out", "botgames")
    games = [g for g in (game(p) for p in sorted(glob.glob(os.path.join(out, "*"))) if os.path.isdir(p)) if g]
    lines = []
    say = lambda s="": lines.append(s)
    say(f"Партий: {len(games)}")
    say("эпоха | карта | стороны (люди, млн) | выпуск А/Б | дивизий к войне А/Б | исход | мес. войны | провинций А, Б: начало→конец | погибло тыс. А/Б | наступлений взято А, Б | недовольство: начало→к войне→конец (А/Б)")
    for g in games:
        b = g["battles"]
        rate = []
        for s in "01":
            att = [x for x in b if x["attacker"] == s]
            rate.append(f"{sum(x['won'] == '1' for x in att)}/{len(att)}")
        fell = "не кончилась" if g["fell"] is None else f"капитулировала {g['names'][g['fell']]}"
        say(
            f"{g['era'][:12]} | {g['seed']} | {g['names'][0]} {g['people'][0]:.0f} — {g['names'][1]} {g['people'][1]:.0f} | {g['gdp'][0]:.0f}/{g['gdp'][1]:.0f} | "
            f"{g['units'][0]:.0f}/{g['units'][1]:.0f} | {fell} | {g['war_months']} | {g['prov0'][0]}→{g['prov'][0]}, {g['prov0'][1]}→{g['prov'][1]} | "
            f"{g['dead'][0]:.0f}/{g['dead'][1]:.0f} | {rate[0]}, {rate[1]} | "
            f"{g['anger_start'][0]:.2f}→{g['anger_peace'][0]:.2f}→{g['anger_end'][0]:.2f} / {g['anger_start'][1]:.2f}→{g['anger_peace'][1]:.2f}→{g['anger_end'][1]:.2f}"
        )
    by = defaultdict(list)
    for g in games:
        by[g["era"]].append(g)
    for era, gs in by.items():
        say()
        say(f"== {era}: {len(gs)} партий")
        ended = [g for g in gs if g["fell"] is not None]
        say(f"кончились капитуляцией: {len(ended)} из {len(gs)}; войны длились (мес.): {sorted(g['war_months'] for g in gs)}")
        big = sum(1 for g in ended if g["gdp"][1 - g["fell"]] >= g["gdp"][g["fell"]])
        say(f"победил тот, у кого выпуск больше: {big} из {len(ended)}")
        ratios = [max(g["gdp"]) / max(min(g["gdp"]), 1e-9) for g in gs]
        say(f"во сколько раз выпуск сильного больше: {', '.join(f'{r:.1f}' for r in ratios)}")
        allb = [x for g in gs for x in g["battles"]]
        if allb:
            won = sum(x["won"] == "1" for x in allb)
            say(f"боёв {len(allb)}, наступление удалось в {won / len(allb) * 100:.0f}%; боёв в месяц войны: {len(allb) / max(1, sum(g['war_months'] for g in gs)):.1f}")
            # how true the staffs' war games were: promised odds against what came
            bins = [(0, 0.5), (0.5, 0.6), (0.6, 0.7), (0.7, 0.8), (0.8, 0.9), (0.9, 1.01)]
            parts = []
            for lo, hi in bins:
                sel = [x for x in allb if lo <= num(x["odds"]) < hi]
                if sel:
                    parts.append(f"{lo:.1f}–{min(hi, 1):.1f}: {sum(x['won'] == '1' for x in sel) / len(sel) * 100:.0f}% из {len(sel)}")
            say("штаб обещал → вышло: " + "; ".join(parts))
            # how alike the sides fight: each side's share of its offensives that succeeded
            rates = []
            for g in gs:
                for s in "01":
                    att = [x for x in g["battles"] if x["attacker"] == s]
                    if len(att) >= 10:
                        rates.append(sum(x["won"] == "1" for x in att) / len(att))
            if rates:
                m = sum(rates) / len(rates)
                sd = (sum((r - m) ** 2 for r in rates) / len(rates)) ** 0.5
                say(f"успех наступлений по сторонам (от 10 боёв): {', '.join(f'{r * 100:.0f}' for r in sorted(rates))}% — разброс ±{sd * 100:.0f} пунктов")
            terr = defaultdict(lambda: [0, 0])
            for x in allb:
                terr[x["terrain"]][0] += 1
                terr[x["terrain"]][1] += x["won"] == "1"
            say("по местности (боёв, удалось): " + "; ".join(f"{t} {n} ({w / n * 100:.0f}%)" for t, (n, w) in sorted(terr.items(), key=lambda kv: -kv[1][0])))
        gens = [x for g in gs for x in g["generals"] if x["won"] + x["lost"] > 0]
        if gens:
            for title, pick in (("смелые (смелость > +0.3)", lambda x: x["bold"] > 0.3), ("средние", lambda x: -0.3 <= x["bold"] <= 0.3), ("осторожные (< −0.3)", lambda x: x["bold"] < -0.3)):
                sel = [x for x in gens if pick(x)]
                if sel:
                    w = sum(x["won"] for x in sel)
                    n = sum(x["won"] + x["lost"] for x in sel)
                    say(f"генералы {title}: {len(sel)} чел., боёв {n} ({n / len(sel):.0f} на генерала), побед {w / max(n, 1) * 100:.0f}%")
            for title, pick in (("талантливые (> 0.6)", lambda x: x["talent"] > 0.6), ("бездарные (< 0.4)", lambda x: x["talent"] < 0.4)):
                sel = [x for x in gens if pick(x)]
                if sel:
                    w = sum(x["won"] for x in sel)
                    n = sum(x["won"] + x["lost"] for x in sel)
                    say(f"генералы {title}: {len(sel)} чел., побед {w / max(n, 1) * 100:.0f}% из {n}")
        say(f"разработок за партию: {', '.join(str(g['designs']) for g in gs)}; волнений (новостей): {', '.join(str(g['unrest']) for g in gs)}")
        drift = ", ".join("%.3f/%.3f" % (g["gdp_peace"][0], g["gdp_peace"][1]) for g in gs)
        say(f"выпуск к началу за годы мира (А/Б): {drift}")
    text = "\n".join(lines)
    print(text)
    open(os.path.join(out, "stats.txt"), "w", encoding="utf-8").write(text + "\n")


if __name__ == "__main__":
    main()
