"""Pictures of the game's window, one after another (each opens the window, waits, saves, closes).

python tools/winshots.py OUTDIR [--exe target/release/play.exe] [--pack industrial] [--seed 3]
                         [--only name1,name2]

Each picture is a set of `play --native --shot` keys (see SETS); a war of 4 months is prepared.
"""
import os
import subprocess
import sys

SETS = {
    "map": [],
    "nation": ["--tab", "nation"],
    "arms": ["--tab", "arms"],
    "econ": ["--tab", "econ"],
    "science": ["--tab", "science"],
    "army": ["--tab", "army"],
    "design": ["--tab", "army", "--design", "0"],
    "politics": ["--tab", "politics"],
    "news": ["--tab", "news"],
    "menu": ["--menu"],
    "keys": ["--menu", "--keys"],
    "near": ["--zoom", "3.2"],
    "pick": ["--zoom", "3.2", "--pick", "--frames", "90"],
    "divs": ["--zoom", "5"],
    "mid": ["--zoom", "1.8"],
    "forest": ["--zoom", "7", "--center", "forest"],
    "forestfar": ["--zoom", "4", "--center", "forest"],
    "marsh": ["--zoom", "7", "--center", "marsh"],
    "hills": ["--zoom", "5", "--center", "hills"],
    "plains": ["--zoom", "5", "--center", "plains"],
    "rangefar": ["--zoom", "2.5", "--center", "range"],
    "marshclose": ["--zoom", "13", "--center", "marsh"],
    "town": ["--zoom", "8", "--center", "town"],
    "close": ["--zoom", "6", "--center", "high"],
    "range": ["--zoom", "6", "--center", "range"],
    "townclose": ["--zoom", "14", "--center", "town"],
    "year3": ["--peace", "30"],
    "terrain": ["--zoom", "2.2", "--mode", "Рельеф"],
    "battle": ["--tab", "battle"],
    "event": ["--event"],
    "narrow": ["--size", "1100x640", "--tab", "econ"],
    "tip_top": ["--hover", "470,30", "--frames", "90"],
    "tip_map": ["--zoom", "1.6", "--hover", "560,420", "--frames", "90"],
    "tip_modes": ["--hover", "1560,872", "--frames", "90"],
    "cursors": ["--cursors", "--hover", "900,600"],
    "plan": ["--choose", "0", "--draft", "--hover", "1500,150", "--frames", "90"],
    "plannear": ["--zoom", "3.2", "--choose", "0", "--draft", "--hover", "1500,150", "--frames", "90"],
    "armies": ["--zoom", "1.4", "--mode", "Армии", "--choose", "1", "--hover", "1500,150", "--frames", "90"],
    "route": ["--zoom", "3.2", "--pick", "--hover", "1100,300", "--frames", "90"],
    # (26th session: the navy, the seas, the army's money — a tall window shows the folder's lower parts)
    "navy": ["--tab", "navy"],
    "navytall": ["--size", "1600x2600", "--tab", "navy"],
    "seas": ["--zoom", "1.1", "--mode", "Моря"],
    "armytall": ["--size", "1600x3600", "--tab", "army"],
    "politicstall": ["--size", "1600x2400", "--tab", "politics"],
    # (28th session: the pages of the economy's and the army's folders; tall — the whole page)
    "econ0": ["--size", "1600x1800", "--tab", "econ:0"],
    "econ1": ["--tab", "econ:1"],
    "econ2": ["--tab", "econ:2"],
    "econ3": ["--tab", "econ:3"],
    "army0": ["--size", "1600x1500", "--tab", "army:0"],
    "army1": ["--tab", "army:1"],
    "army2": ["--tab", "army:2"],
    "army3": ["--tab", "army:3"],
    "army4": ["--tab", "army:4"],
}


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 2
    out = args[0]
    opt = {}
    i = 1
    while i < len(args):
        opt[args[i]] = args[i + 1]
        i += 2
    exe = os.path.abspath(opt.get("--exe", "target/release/play.exe"))
    pack = opt.get("--pack", "industrial")
    seed = opt.get("--seed", "3")
    only = opt.get("--only")
    names = only.split(",") if only else list(SETS)
    os.makedirs(out, exist_ok=True)
    bad = 0
    for name in names:
        extra = SETS.get(name)
        if extra is None:
            print(f"нет такого снимка: {name}")
            bad += 1
            continue
        frames = opt.get("--frames", "60")
        cmd = [exe, "--native", "--pack", f"packs/{pack}.toml", "--seed", seed, "--prep-war", "4",
               "--shot", os.path.join(out, f"{name}.png"), "--frames", frames] + extra
        r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
        line = [l for l in r.stdout.splitlines() if "снимок" in l]
        print(name, "—", line[0] if line else ("ОШИБКА " + r.stderr[-400:]))
        bad += r.returncode != 0
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
