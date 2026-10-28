"""Make text files CRLF throughout (lone LF -> CRLF) and say what changed.

    python tools/crlf.py FILE [FILE ...]
    python tools/crlf.py --rust          (every .rs file under src/ and build.rs)
"""
import glob
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
paths = sys.argv[1:]
if paths == ["--rust"]:
    paths = glob.glob(os.path.join(ROOT, "src", "**", "*.rs"), recursive=True) + [os.path.join(ROOT, "build.rs")]
for path in paths:
    data = open(path, "rb").read()
    fixed = data.replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
    if fixed != data:
        open(path, "wb").write(fixed)
        lone = data.count(b"\n") - data.count(b"\r\n")
        print(f"{os.path.relpath(path, ROOT)}: {lone} lone LF fixed")
