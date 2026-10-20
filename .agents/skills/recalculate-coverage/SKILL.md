---
name: recalculate-coverage
description: Recalculate how much of the original Exanima (Exarust's component map) is reimplemented, audit whether a published percentage is trustworthy, and regenerate docs/COMPONENT-MAP.md plus both docs/img/coverage-map-*.png when ratings change. Use whenever the user asks for the coverage percentage, the component map, "how much of the game is done", a coverage review, or asks to update the coverage-map images — and also when a commit changes behavior and someone is about to quote the current headline.
license: Apache-2.0
compatibility: Python 3, git, and the repo's research tree under context/ (git-ignored). The image step needs Playwright's headless Chromium shell.
---

# Recalculate the Exarust coverage

Coverage here is a **measurement**, not a summary: the share of the retail
`Exanima.exe`'s code, by bytes, whose behaviour Exarust reproduces. The page and
the README images are generated artifacts. Everything below exists because
those artifacts have repeatedly drifted from their inputs — a percentage copied
from a plan, a citation to a file that no longer exists, a 53% component
silently dropped from a section total, a document and its images built at
different times. Treat the number as guilty until the audit says otherwise.

## The shape of the problem

```
context/re/component-map/          git-ignored research inputs
  sections.json       inventory: every original function → component → section
  ratings.json        the base rating per component
  fixes.json          corrections, splits, moves, combat-rule sub-ratings
  functions.json      byte sizes
context/tools/         generators (they hardcode the repo root)
  build_map_data.py   → map/data.json
  component_map_doc.py→ docs/COMPONENT-MAP.md
  coverage_image.py   → docs/img/coverage-map-{dark,light}.png
```

The inputs are untracked while the outputs are tracked. That asymmetry is the
root of most bad numbers: the page can be committed without the data that
produced it, and the data can be edited without the page.

## Non-negotiables

1. **A rating must point at code that exists and at behaviour you checked.**
   Copying a percentage out of `docs/PLAN.md` or out of the page's own prose is
   how a guess becomes a headline. The prose is a claim; the rating is the
   measurement. Read `references/rating-rubric.md` before assigning any value.
2. **Never hand-edit `map/data.json`.** It is generated. A rating that exists
   only there will be silently reverted by the next rebuild — the audit calls
   this out as a divergence, and the runner refuses to proceed when sources
   would lower an already-published rating.
3. **Status is a byte band, not a completeness claim.** `>= 80` is labelled
   "reimplemented" even when a named feature is missing; always say which
   feature is missing in the text so a reader is not misled by the label.
4. **The page and both images are one artifact.** Never publish one without
   the other two, and never describe a figure as final while a section is
   obviously unrated (a section at 0% whose crate lists it is a stale table,
   not a real zero).
5. **Record who measured what.** The history row carries the commit, whether
   the tree was dirty, and whether HEAD moved mid-run. A figure measured on a
   dirty tree is a snapshot, not a release.

## Workflow

### 1. Freeze and look

```bash
git log -3 --oneline
git status --short
python3 .agents/skills/recalculate-coverage/scripts/audit_coverage.py --check
```

The audit is read-only and exits non-zero on hard errors. Resolve every
**source** error before rebuilding:

- a cited path that does not exist → fix the citation, not the checker;
- a source rating above 0% with no code listed → add the evidence or lower it;
- no hard error, only warnings → read them; they are the honest caveats.

If the tree is dirty or HEAD moves during the work, say so. This repository is
often edited concurrently; a figure computed mid-edit is valid only for that
state.

### 2. Find what actually changed

Only rerate components whose behaviour changed. Look at:

```bash
git log --oneline <last-covered-commit>..HEAD
git diff --stat <last-covered-commit>..HEAD -- crates
```

Map each code change to the **original** component it reproduces, not to the
crate it lives in. A new rule in `exa_ai/src/path.rs` credits `TPathFinder`; a
new render pass credits whatever original renderer component it mirrors. The
inventory is the authority on where a function belongs.

### 3. Rerate

For each component:

1. Read the original's role and size in `sections.json` / `functions.json`
   (and, when available, the section notes under `context/re/component-map/sections/`).
2. List the original rules, mark each reproduced / partial / missing.
3. Weigh by bytes (the map's rule), not by how much code you wrote. A small
   faithful rule is worth more than a large approximation.
4. Write the evidence: file paths that exist, the function names that carry the
   behaviour, and the explicit gap. Name the gap even when the rating is high.
5. Put it in `ratings.json` (base) or `fixes.json` (correction, with a note
   saying what it supersedes).

See `references/rating-rubric.md` for the weighting convention, worked
examples, and the failure modes to avoid.

### 4. Rebuild atomically

```bash
python3 .agents/skills/recalculate-coverage/scripts/recalc_coverage.py \
  --message "pathfinder layer, cloth citation"
```

The runner:

- audits the **sources** and refuses on errors;
- prints every pending rating change and refuses when the sources would
  *lower* a rating the generated data already carries (that is how hand-patched
  work gets deleted), unless `--allow-reverts`;
- rebuilds `data.json`, the page and both PNGs;
- audits the **artifacts** (status coherence, published citations, doc/data
  agreement, image freshness, data-ahead-of-sources);
- appends a dated row to `docs/reviews/coverage-history.md` only if the
  postflight is clean.

Useful variants: `--check` (audit only), `--no-images` (page only, during
debugging), `--force` (publish despite errors — the audit still runs).

### 5. Read the result before quoting it

- If the audit reports a section below 1% whose crate lists it, the section is
  unrated. Either rerate it or state that the headline is a **floor**
  ("at least X%"), because the aggregate mixes corrected and stale sections.
- Prefer a range over a false precision when large sections are unrated.
- Check that the largest-missing and best-covered tables make sense: a 53%
  component cannot also be "missing".

### 6. Publish

Commit the page, **both** images and the history row together. The commit
message should say what moved (component names and old → new), not just
"update coverage".

State the figure with its basis:

> **17.1% by live code size** (5.7% engine-provided), rated at `<commit>`
> with a dirty tree. Sections `modes` and `ui` are unrated, so this is a
> floor, not the final value.

## Reporting template

```
Coverage: <X>% reimplemented, <Y>% engine-provided (by live code size).
Basis: <commit>, <clean|dirty>, <date>. Combat rules: <Z>%.
Changed: <component> <old>% -> <new>% (why), ...
Caveats: <unrated sections / divergence / untracked inputs>.
Artifacts: docs/COMPONENT-MAP.md + both coverage-map PNGs, regenerated together.
```

## Failure modes this skill exists to catch

- **Prose laundering** — taking a number from `PLAN.md` into the data.
- **Dead citations** — a path in the evidence that no longer exists (the audit
  checks both sources and the published page).
- **Silent reverts** — data hand-patched ahead of its sources; the next rebuild
  deletes the work.
- **Status drift** — a stored status that disagrees with its percentage.
- **Phantom zeros** — a section at 0% while real code and crate citations exist.
- **Split artifacts** — page newer than images, or images rendered from a
  different data revision.
- **Sub-band credit** — components rated 1–4% contribute nothing by the map's
  own rule; decide between 0 and a real ≥5% rating instead of leaving a
  misleading 3%.
- **Concurrent writers** — HEAD moving mid-run; the history row must say so.

## Files

- `scripts/audit_coverage.py` — read-only checks; `--scope source|artifacts|all`, `--json`.
- `scripts/recalc_coverage.py` — preflight, rebuild, postflight, history row.
- `references/rating-rubric.md` — how to choose a percentage and write evidence.
