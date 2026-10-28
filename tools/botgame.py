"""Games of bots (`play --auto`): both sides played by bots, each game written down in its folder.

    python tools/botgame.py [--pack packs/x.toml ...] [--games N] [--seed N] [--war-at 36]
                            [--war-months 60]

Each game: out/botgames/<pack>_<map>/chronicle.txt (in words), months.csv, battles.csv,
map_MMM.png (the front); a line in out/botgames/summary.txt. A game runs as fast as the world is
reckoned. Games run one after another (one heavy job at a time), from a COPY of the exe, so cargo
can build meanwhile.
"""
import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def one(args, exe, pack, seed):
    cmd = [exe, "--auto", "--war-at", str(args.war_at), "--war-months", str(args.war_months), "--pack", pack]
    if seed is not None:
        cmd += ["--seed", str(seed)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = r.stdout.strip().splitlines()
    print(f"  {out[2] if len(out) > 2 else ''} / {out[-1] if out else r.stderr[-300:]}  ({time.time() - t0:.0f} с)")


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser()
    ap.add_argument("--pack", action="append", default=[])
    ap.add_argument("--games", type=int, default=1, help="games of each pack")
    ap.add_argument("--seed", type=int)
    ap.add_argument("--war-at", type=int, default=36)
    ap.add_argument("--war-months", type=int, default=60)
    ap.add_argument("--exe", default=os.path.join(ROOT, "target", "release", "play.exe"))
    args = ap.parse_args()
    packs = args.pack or ["packs/industrial.toml", "packs/musket.toml", "packs/wasteland.toml"]
    tmp = tempfile.mkdtemp(prefix="botgame_")
    exe = os.path.join(tmp, "play_bots.exe")
    shutil.copy(args.exe, exe)
    try:
        for pack in packs:
            for g in range(args.games):
                seed = args.seed + g if args.seed is not None else None
                print(f"{pack}, партия {g + 1} из {args.games}{'' if seed is None else f', карта {seed}'}")
                one(args, exe, pack, seed)
                # (a fresh map by the clock: let it tick)
                time.sleep(1.1)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
