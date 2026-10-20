#!/usr/bin/env python3
"""Recalculate the coverage figure and regenerate the map artifacts atomically.

The map is generated from research data that lives in the git-ignored
``context/`` tree, while the page and the two README images are tracked. That
split is the main way the published number drifts away from its inputs, so
this runner does four things in one shot:

1. audits the inputs *before* rebuilding (a citation that does not exist, or a
   status that disagrees with its percentage, must be fixed first — rebuilding
   would only hide it in a fresh-looking page);
2. rebuilds ``map/data.json``, ``docs/COMPONENT-MAP.md`` and both theme PNGs;
3. audits the result, including whether the images are newer than the data and
   whether the document's headline/section rows match;
4. appends a dated row to ``docs/reviews/coverage-history.md`` with the figure,
   the commit, whether the tree was dirty, and a per-component diff.

It never commits. Publishing stays a human decision, made after reading the
diff and the audit warnings.

Usage:
    python3 .agents/skills/recalculate-coverage/scripts/recalc_coverage.py --check
    python3 .agents/skills/recalculate-coverage/scripts/recalc_coverage.py
    python3 .agents/skills/recalculate-coverage/scripts/recalc_coverage.py --message "cloth + morphing"
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
AUDIT = HERE / "audit_coverage.py"
HISTORY_REL = Path("docs/reviews/coverage-history.md")
DATA_REL = Path("context/re/component-map/map/data.json")


def run(cmd: list[str], root: Path, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=root, text=True, check=check)


def git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=False
    ).stdout.strip()


def load_components(root: Path) -> dict:
    path = root / DATA_REL
    if not path.exists():
        return {}
    data = json.loads(path.read_text())
    return {c["n"]: float(c.get("c") or 0) for s in data["sections"] for c in s["components"]}


def source_coverage(root: Path) -> dict:
    """Coverage per component as the rating sources define it."""
    mapdir = root / "context/re/component-map"
    out = {}
    for section in json.loads((mapdir / "ratings.json").read_text())["sections"].values():
        for name, rating in section["ratings"].items():
            out[name] = float(rating.get("coverage") or 0)
    fixes_path = mapdir / "fixes.json"
    if fixes_path.exists():
        for rating in json.loads(fixes_path.read_text()).get("ratings", []):
            out[rating["name"]] = float(rating.get("coverage") or 0)
    return out


def headline(root: Path) -> dict:
    data = json.loads((root / DATA_REL).read_text())
    return data["coverage"]


def history_row(root: Path, cov: dict, commit: str, dirty: bool, moved: bool, message: str) -> str:
    date = dt.date.today().isoformat()
    dirty_mark = "dirty" if dirty else "clean"
    moved_mark = ", HEAD moved during run" if moved else ""
    note = message.replace("|", "/") if message else ""
    return (
        f"| {date} | {commit} | {dirty_mark}{moved_mark} | "
        f"{cov['bytes']['reimplemented']}% | {cov['bytes']['engine']}% | "
        f"{cov['functions']['reimplemented']}% | {cov['combat_rules']['reimplemented']}% | {note} |\n"
    )


def append_history(root: Path, row: str) -> Path:
    path = root / HISTORY_REL
    header = (
        "# Coverage history\n\n"
        "Every row is a generated recalculation: the artifact set in the same commit\n"
        "(`docs/COMPONENT-MAP.md` and both `docs/img/coverage-map-*.png`) matches it.\n"
        "`dirty` means uncommitted work was rated; `HEAD moved` means the tree changed\n"
        "while the map was being built, so treat the row as a best-effort snapshot.\n\n"
        "| date | commit | state | reimplemented | engine | by functions | combat rules | note |\n"
        "|---|---|---|---:|---:|---:|---:|---|\n"
    )
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(header)
    with path.open("a") as fh:
        fh.write(row)
    return path


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", default=None, help="repository root (default: git toplevel)")
    ap.add_argument("--check", action="store_true", help="audit only; do not rebuild")
    ap.add_argument("--force", action="store_true", help="rebuild even if the preflight audit fails")
    ap.add_argument("--no-images", action="store_true", help="rebuild the page but not the PNGs")
    ap.add_argument("--message", default="", help="short note for the history row")
    ap.add_argument(
        "--allow-reverts",
        action="store_true",
        help="proceed even when the sources would lower an already-published rating",
    )
    args = ap.parse_args()

    root = Path(args.root) if args.root else Path(git(Path.cwd(), "rev-parse", "--show-toplevel") or ".")
    py = sys.executable

    if args.check:
        return subprocess.run([py, str(AUDIT), "--root", str(root)]).returncode

    commit_before = git(root, "rev-parse", "--short", "HEAD") or "unknown"
    dirty = bool(git(root, "status", "--porcelain"))
    before = load_components(root)

    print("== preflight audit ==")
    pre = subprocess.run(
        [py, str(AUDIT), "--root", str(root), "--scope", "source"],
        text=True,
        capture_output=True,
    )
    print(pre.stdout.rstrip() or pre.stderr.rstrip())
    if pre.returncode != 0 and not args.force:
        print(
            "\nrefusing to rebuild: fix the errors above, or pass --force to publish a\n"
            "page that still carries them (the audit will run again afterwards).",
            file=sys.stderr,
        )
        return 1

    pending = [
        (name, before.get(name, 0.0), value)
        for name, value in source_coverage(root).items()
        if name in before and abs(before[name] - value) > 0.001
    ]
    if pending:
        print("\n== pending rating changes ==")
        for name, old, new in sorted(pending, key=lambda item: -(abs(item[2] - item[1]))):
            print(f"  {name}: {old:g}% -> {new:g}%")
        reverts = [p for p in pending if p[2] < p[1]]
        if reverts and not args.allow_reverts:
            print(
                "\nrefusing to rebuild: the sources would undo a rating the generated data\n"
                "already carries. Find the source that produced it (or accept the loss with\n"
                "--allow-reverts), otherwise the rebuild silently deletes that work.",
                file=sys.stderr,
            )
            return 1

    print("\n== rebuild ==")
    for script in ("context/tools/build_map_data.py", "context/tools/component_map_doc.py"):
        print(f"$ {script}")
        done = run([py, script], root, check=False)
        if done.returncode != 0:
            print(f"generator failed: {script}", file=sys.stderr)
            return done.returncode
    if not args.no_images:
        print("$ context/tools/coverage_image.py")
        done = run([py, "context/tools/coverage_image.py"], root, check=False)
        if done.returncode != 0:
            print("image regeneration failed", file=sys.stderr)
            return done.returncode

    print("\n== postflight audit ==")
    post = subprocess.run([py, str(AUDIT), "--root", str(root)], text=True, capture_output=True)
    print(post.stdout.rstrip() or post.stderr.rstrip())

    after = load_components(root)
    changed = sorted(
        ((name, before.get(name, 0.0), after[name]) for name in after if before.get(name, 0.0) != after[name]),
        key=lambda item: -(abs(item[2] - item[1])),
    )
    if changed:
        print("\n== components that moved ==")
        for name, old, new in changed:
            print(f"  {name}: {old:g}% -> {new:g}%")
    else:
        print("\nno component rating changed; the headline should be unchanged")

    if post.returncode != 0:
        print(
            "\nnot recorded in the history: the audit still reports errors. Fix them and run\n"
            "again; the regenerated page and images stay in the tree for inspection.",
            file=sys.stderr,
        )
        return post.returncode

    cov = headline(root)
    commit_after = git(root, "rev-parse", "--short", "HEAD") or "unknown"
    moved = commit_before != commit_after
    row = history_row(root, cov, commit_before, dirty, moved, args.message)
    path = append_history(root, row)
    print(f"\nappended to {path.relative_to(root)}:")
    print(row.rstrip())
    if moved:
        print(
            f"\nWARNING: HEAD moved during the run ({commit_before} -> {commit_after}). "
            "Re-run after the concurrent work settles.",
            file=sys.stderr,
        )
    return post.returncode


if __name__ == "__main__":
    sys.exit(main())
