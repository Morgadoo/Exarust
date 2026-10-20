#!/usr/bin/env python3
"""Audit the Exarust coverage map before anyone publishes a headline.

The coverage figure is a measurement, so the inputs have to survive a few
mechanical checks before the number is quoted:

* every cited Exarust path exists;
* a component's status agrees with its percentage (the map's own rule);
* components carrying credit are not silently excluded from the totals;
* the tracked document's headline and section rows match the generated data;
* the two PNGs are not older than the data they are supposed to draw;
* the inputs are reachable from a clean checkout (they live under the
  git-ignored ``context/`` tree);
* no section is reported as 0% while a crate claims to implement it.

Exit status is 1 when a hard error is found, 0 otherwise. Warnings are
printed but do not fail the run: they are judgement calls that the operator
must resolve (or accept in writing) before publishing.

Usage:
    python3 .agents/skills/recalculate-coverage/scripts/audit_coverage.py
    python3 .agents/skills/recalculate-coverage/scripts/audit_coverage.py --json
    python3 .agents/skills/recalculate-coverage/scripts/audit_coverage.py --root /path
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

MAP_REL = Path("context/re/component-map")
DOC_REL = Path("docs/COMPONENT-MAP.md")
IMG_REL = [Path("docs/img/coverage-map-dark.png"), Path("docs/img/coverage-map-light.png")]
PATH_RE = re.compile(r"\b((?:crates|docs|context|\.agents)/[\w./+-]+\.\w+)\b")
DOC_HEADLINE = re.compile(r"\*\*([\d.]+)% reimplemented\*\*")
DOC_ENGINE = re.compile(r"\*\*([\d.]+)% provided\*\*")
DOC_ROW = re.compile(r"^\| ([^|]+?) \| [\d,]+ \| [\d,]+ \| \d+ \| ([\d.]+)% \| ([\d.]+)% \|", re.M)
RULE_BAND = 5.0  # below this the map calls a component "missing" and credits nothing


def git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=False
    ).stdout.strip()


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def load(root: Path, rel: Path):
    with (root / rel).open() as fh:
        return json.load(fh)


def merged_ratings(mapdir: Path) -> dict:
    """Base ratings overridden by fixes.json, the way build_map_data.py reads them."""
    out = {}
    for section in json.loads((mapdir / "ratings.json").read_text())["sections"].values():
        out.update(section["ratings"])
    fixes_path = mapdir / "fixes.json"
    if fixes_path.exists():
        fixes = json.loads(fixes_path.read_text())
        for rating in fixes.get("ratings", []):
            out[rating["name"]] = rating
        for name, evidence in fixes.get("evidence", {}).items():
            if name in out:
                out[name] = {**out[name], "evidence": evidence}
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", default=None, help="repository root (default: git toplevel)")
    ap.add_argument("--json", action="store_true", help="emit a machine-readable report")
    ap.add_argument(
        "--scope",
        choices=("all", "source", "artifacts"),
        default="all",
        help="source: only rating inputs (used before a rebuild); artifacts: "
             "only the generated page and images (used after one); all: both",
    )
    args = ap.parse_args()

    root = Path(args.root) if args.root else Path(git(Path.cwd(), "rev-parse", "--show-toplevel") or ".")
    mapdir = root / MAP_REL
    doc_path = root / DOC_REL
    source_errors: list[str] = []
    artifact_errors: list[str] = []
    warnings: list[str] = []
    notes: list[str] = []

    # ---- inputs -----------------------------------------------------------------
    missing = [p for p in (
        mapdir / "sections.json",
        mapdir / "ratings.json",
        mapdir / "functions.json",
        mapdir / "map/data.json",
        doc_path,
        *[root / p for p in IMG_REL],
    ) if not p.exists()]
    if missing:
        names = ", ".join(str(p.relative_to(root)) for p in missing)
        if args.json:
            print(json.dumps({"errors": [f"missing inputs: {names}"]}, indent=2))
        else:
            print(f"ERROR: missing inputs: {names}")
        return 1

    data = load(root, MAP_REL / "map/data.json")
    ratings = merged_ratings(mapdir)

    # ---- 1. citations exist -----------------------------------------------------
    # Source citations gate a rebuild; published citations only prove the page is
    # stale, which a rebuild fixes.
    def dead_paths(entries: dict) -> list[tuple[str, str]]:
        found = []
        for name, citations in entries.items():
            for citation in citations or []:
                for rel in PATH_RE.findall(citation):
                    if not (root / rel).exists():
                        found.append((name, rel))
        return found

    for name, rel in dead_paths({n: r.get("exarust") for n, r in ratings.items()}):
        source_errors.append(f"{name}: cites a path that does not exist: {rel}")
    published = {
        f"{s['id']}/{c['n']}": c.get("x", []) for s in data["sections"] for c in s["components"]
    }
    for name, rel in dead_paths(published):
        artifact_errors.append(f"{name} (published): cites a path that does not exist: {rel}")

    # ---- 1b. a source rating that claims credit without evidence ----------------
    for name, rating in ratings.items():
        cov = float(rating.get("coverage") or 0)
        if rating.get("status") != "not-applicable" and cov > 0 and not rating.get("exarust"):
            source_errors.append(f"{name}: rated {cov:g}% with no Exarust code listed in the source")
        if cov == 0 and rating.get("exarust"):
            warnings.append(
                f"{name}: names Exarust code at 0% in the source; the page will drop those citations"
            )

    # ---- 2. status agrees with the percentage ----------------------------------
    def coverage_of(component) -> float:
        return float(component.get("c") or 0)

    def expected_status(component) -> str:
        if component["s"] in ("engine", "not-applicable"):
            return component["s"]
        cov = coverage_of(component)
        return "done" if cov >= 80 else "partial" if cov >= RULE_BAND else "missing"

    for section in data["sections"]:
        for c in section["components"]:
            want = expected_status(c)
            if c["s"] != want:
                artifact_errors.append(
                    f"{c['n']}: stored status '{c['s']}' but {coverage_of(c)}% implies "
                    f"'{want}' ({c['b']} bytes)"
                )
            if coverage_of(c) > 0 and not c["x"]:
                artifact_errors.append(f"{c['n']}: {coverage_of(c)}% with no Exarust code listed")
            if coverage_of(c) == 0 and c["x"]:
                artifact_errors.append(f"{c['n']}: lists Exarust code at 0%")

    # ---- 3. credit that the <5% rule drops --------------------------------------
    dropped = [
        (c["n"], c["c"], c["b"]) for s in data["sections"] for c in s["components"]
        if 0 < coverage_of(c) < RULE_BAND and c["s"] != "not-applicable"
    ]
    if dropped:
        notes.append(
            "components below the 5% band contribute nothing to the totals "
            f"({len(dropped)}): " + ", ".join(f"{n} {c}%" for n, c, _ in dropped[:8])
            + (" …" if len(dropped) > 8 else "")
        )

    # ---- 4. document matches the generated data ---------------------------------
    doc = doc_path.read_text()
    computed = data["coverage"]["bytes"]
    head = DOC_HEADLINE.search(doc)
    eng = DOC_ENGINE.search(doc)
    if not head or abs(float(head.group(1)) - computed["reimplemented"]) > 0.05:
        artifact_errors.append(
            f"headline drift: document says {head.group(1) if head else 'nothing'}%, "
            f"data says {computed['reimplemented']}%"
        )
    if not eng or abs(float(eng.group(1)) - computed["engine"]) > 0.05:
        artifact_errors.append(
            f"engine drift: document says {eng.group(1) if eng else 'nothing'}%, "
            f"data says {computed['engine']}%"
        )
    rows = {m.group(1).strip(): (float(m.group(2)), float(m.group(3))) for m in DOC_ROW.finditer(doc)}
    for section in data["sections"]:
        title = section["title"]
        if title not in rows:
            artifact_errors.append(f"section row missing from the document: {title}")
            continue
        live = [c for c in section["components"] if c["s"] != "not-applicable"]
        base = sum(c["b"] for c in live) or 1
        own = sum(c["b"] * coverage_of(c) / 100 for c in live if c["s"] in ("done", "partial")) / base * 100
        engine = sum(c["b"] * coverage_of(c) / 100 for c in live if c["s"] == "engine") / base * 100
        got_own, got_engine = rows[title]
        if abs(got_own - own) > 1.0 or abs(got_engine - engine) > 1.0:
            artifact_errors.append(
                f"section drift {title}: document {got_own:.0f}%/{got_engine:.0f}%, "
                f"data {own:.0f}%/{engine:.0f}%"
            )

    # ---- 4b. the generated data is not ahead of its sources ---------------------
    # A generated rating that no source carries will be silently reverted by the
    # next rebuild. This is how hand-patched data loses work, so it is an error.
    source_cov = {
        name: float(rating.get("coverage") or 0)
        for name, rating in ratings.items()
        if rating.get("status") != "not-applicable"
    }
    for section in data["sections"]:
        for c in section["components"]:
            if c["s"] in ("engine", "not-applicable") or c["n"] not in source_cov:
                continue
            if abs(coverage_of(c) - source_cov[c["n"]]) > 0.001:
                artifact_errors.append(
                    f"{c['n']}: the generated data says {coverage_of(c):g}% but the sources say "
                    f"{source_cov[c['n']]:g}%; a rebuild would silently change it"
                )

    # ---- 5. images are not older than their data --------------------------------
    newest_input = max(
        (mapdir / "map/data.json").stat().st_mtime,
        (mapdir / "ratings.json").stat().st_mtime,
        doc_path.stat().st_mtime,
    )
    for rel in IMG_REL:
        image = root / rel
        if image.stat().st_mtime + 1 < newest_input:
            artifact_errors.append(
                f"{rel} is older than the data/document it draws; regenerate both themes"
            )

    # ---- 6. a 0% section that a crate claims to implement -----------------------
    # A warning, not an error: it means the section table is stale and needs a
    # rerate, but publishing a provisional figure is still allowed when the page
    # says so. Hiding it would be worse than publishing it with the caveat.
    section_pct = {}
    for section in data["sections"]:
        live = [c for c in section["components"] if c["s"] != "not-applicable"]
        base = sum(c["b"] for c in live) or 1
        section_pct[section["id"]] = sum(
            c["b"] * coverage_of(c) / 100 for c in live if c["s"] in ("done", "partial")
        ) / base * 100
    for crate in data.get("crates", []):
        for sid in crate.get("sections", []):
            if sid in section_pct and section_pct[sid] < 1.0:
                warnings.append(
                    f"section '{sid}' is {section_pct[sid]:.1f}% but crate "
                    f"'{crate['n']}' lists it among its main sections; rerate is stale"
                )

    # ---- 7. provenance ----------------------------------------------------------
    head_sha = git(root, "rev-parse", "--short", "HEAD") or "unknown"
    dirty = git(root, "status", "--porcelain")
    tracked_rating = git(root, "ls-files", "--error-unmatch", str(MAP_REL / "ratings.json"))
    if not tracked_rating:
        warnings.append(
            "rating inputs are untracked (context/ is git-ignored): the committed "
            "document and images cannot be regenerated from a clean checkout"
        )
    if dirty:
        warnings.append(
            "working tree is dirty at audit time; the figure describes an uncommitted state:\n    "
            + dirty.replace("\n", "\n    ")
        )
    notes.append(f"audited at {head_sha}; data sha256 {sha256(mapdir / 'map/data.json')[:16]}")

    # ---- report -----------------------------------------------------------------
    if args.scope == "source":
        errors = source_errors
    elif args.scope == "artifacts":
        errors = artifact_errors
    else:
        errors = source_errors + artifact_errors
    report = {
        "headline": computed,
        "functions": data["coverage"]["functions"],
        "combat_rules": data["coverage"]["combat_rules"],
        "sections": {s["id"]: round(p, 1) for s, p in zip(data["sections"], section_pct.values())},
        "errors": errors,
        "source_errors": source_errors,
        "artifact_errors": artifact_errors,
        "warnings": warnings,
        "notes": notes,
    }
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print(f"coverage: {computed['reimplemented']}% reimplemented, "
              f"{computed['engine']}% engine-provided "
              f"({data['coverage']['functions']['reimplemented']}% by function count)")
        print(f"sections: " + ", ".join(f"{k} {v:g}%" for k, v in report["sections"].items()))
        for label, items in (("ERROR", errors), ("WARN", warnings), ("NOTE", notes)):
            for item in items:
                print(f"{label}: {item}")
        if not errors and not warnings:
            print("clean: inputs, document, images and provenance all agree")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
