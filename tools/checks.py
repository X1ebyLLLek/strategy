"""All the game's self-checks, one suite after another (one heavy job at a time), from a COPY of the
two programs — `checks` (the engine's suites: `checks battle|doctrine|front|corps|nation|trade|advisor|
homefront …`) and `play` (the game) — so cargo can build meanwhile. Logs go to OUT/<suite>.log, the summary to OUT/summary.txt.

    python tools/checks.py OUT [--only SUBSTR[,SUBSTR...]] [--compare OLD_OUT] [--bin DIR]

  --only     run only the suites whose names contain one of the substrings (e.g. `war,front`);
  --compare  after the run, compare each log with the same log of an older run, lines with timings
             left out (to see that a change kept the world bit for bit); prints SAME / DIFFERENT;
  --list     print the suites and exit.

Suites and their usual time: war_* (seconds), trade_toys + society_* + trade_*_* (~2 min all),
front (~1 min), front_groups_* (groups of armies, ~2.5 min each), corps_* (the war of divisions, ~30 s each), doctrine_* (~1-2 min each), advisor_* (~1.5 min), home_* and home<k>_* (~2 min each),
play_selftest (~15 s; speeds are measured in real time — run on an idle machine), window_* (the
game's window drawn without a screen, ~15 s each; needs play.exe built with `--features native`;
the spoiled control: `play --native --uitest --control`, every check must fail), save_* (a game of
chance saved and loaded, ~1.5 min each).
"""
import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ERAS = ["industrial", "musket", "wasteland"]


def suites():
    s = []
    for e in ERAS:
        s.append((f"war_{e}", ["checks", "battle", "--pack", f"packs/{e}.toml"]))
    s.append(("trade_toys", ["checks", "trade"]))
    for n in (1, 2, 3):
        s.append((f"society_{n}", ["checks", "nation", "--seed", str(n)]))
    for e in ERAS:
        for n in (1, 2, 3):
            s.append((f"trade_{e}_{n}", ["checks", "trade", "--map", f"packs/{e}.toml", str(n)]))
    s.append(("front", ["checks", "front"]))
    for e in ERAS:
        s.append((f"front_groups_{e}", ["checks", "front", "--groups", "--pack", f"packs/{e}.toml"]))
    # the war of divisions (as in HOI4), ~30 s each
    for e in ERAS:
        s.append((f"corps_{e}", ["checks", "corps", "--checks", "--pack", f"packs/{e}.toml"]))
    for e in ERAS:
        s.append((f"doctrine_{e}", ["checks", "doctrine", "--pack", f"packs/{e}.toml"]))
    for e in ERAS:
        s.append((f"advisor_{e}", ["checks", "advisor", "--pack", f"packs/{e}.toml"]))
    for e in ERAS:
        s.append((f"home_{e}", ["checks", "homefront", "--pack", f"packs/{e}.toml"]))
    for k in ("army", "econ", "science", "capture", "scientist", "poor"):
        for e in ERAS:
            s.append((f"home{k}_{e}", ["checks", "homefront", "--pack", f"packs/{e}.toml", f"--{k}"]))
    # the law of the draft: where armies are large enough for its ceilings to bind (the industrial age;
    # in the others the armies stay under them — see docs/homefront.md), ~2 min
    s.append(("homedraft_industrial", ["checks", "homefront", "--pack", "packs/industrial.toml", "--draft"]))
    s.append(("play_selftest", ["play", "--selftest"]))
    # the game's window drawn without a screen (the play built with `--features native`, as
    # play.bat builds it), and its spoiled control: there every check must fail
    for e in ERAS:
        s.append((f"window_{e}", ["play", "--native", "--uitest", "--pack", f"packs/{e}.toml", "--seed", "3"]))
    # a game of chance saved and loaded: the same world to the bit (~1.5 min each)
    for e in ERAS:
        s.append((f"save_{e}", ["play", "--savetest", "--pack", f"packs/{e}.toml", "--seed", "3"]))
    return s


TIMING = re.compile(r"\(\d+([.,]\d+)? ?с\)|\d+([.,]\d+)? ?м?с\b|\d+ ?ms|elapsed|секунд")


def strip(text):
    return [l for l in text.splitlines() if not TIMING.search(l)]


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser()
    ap.add_argument("out", nargs="?")
    ap.add_argument("--only")
    ap.add_argument("--compare")
    ap.add_argument("--bin", default=os.path.join(ROOT, "target", "release"))
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    todo = suites()
    if args.only:
        keys = args.only.split(",")
        todo = [t for t in todo if any(k in t[0] for k in keys)]
    if args.list or not args.out:
        for name, cmd in todo:
            print(f"{name:24} {' '.join(cmd)}")
        return
    os.makedirs(args.out, exist_ok=True)
    bindir = tempfile.mkdtemp(prefix="checks_bin_")
    for exe in {c[1][0] for c in todo}:
        shutil.copy(os.path.join(args.bin, exe + ".exe"), os.path.join(bindir, exe + ".exe"))
    total_ok = total_fail = 0
    summary = open(os.path.join(args.out, "summary.txt"), "w", encoding="utf-8")
    try:
        for name, cmd in todo:
            t0 = time.time()
            with open(os.path.join(args.out, name + ".log"), "w", encoding="utf-8", errors="replace") as log:
                r = subprocess.run([os.path.join(bindir, cmd[0] + ".exe")] + cmd[1:], cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
            text = open(os.path.join(args.out, name + ".log"), encoding="utf-8", errors="replace").read()
            ok, fail = text.count("[OK]"), text.count("[FAIL]")
            total_ok += ok
            total_fail += fail
            line = f"{name:24} ok={ok:<3} fail={fail:<2} exit={r.returncode} {time.time() - t0:.0f}s"
            print(line, flush=True)
            summary.write(line + "\n")
            for l in text.splitlines():
                if "[FAIL]" in l:
                    print("    " + l.strip()[:200])
        line = f"TOTAL ok={total_ok} fail={total_fail}"
        print(line)
        summary.write(line + "\n")
    finally:
        summary.close()
        shutil.rmtree(bindir, ignore_errors=True)
    if args.compare:
        same = True
        for name, _ in todo:
            a, b = os.path.join(args.compare, name + ".log"), os.path.join(args.out, name + ".log")
            if not os.path.exists(a):
                print(f"{name:24} (no old log)")
                continue
            sa = strip(open(a, encoding="utf-8", errors="replace").read())
            sb = strip(open(b, encoding="utf-8", errors="replace").read())
            verdict = "SAME" if sa == sb else "DIFFERENT"
            same &= sa == sb
            print(f"{name:24} {verdict}")
        print("all same" if same else "some logs differ")


if __name__ == "__main__":
    main()
