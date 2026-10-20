# Rating rubric

How to turn "Exarust implements some of this component" into a number that
survives an audit.

## What is being measured

The share of the **original** component's code whose behaviour Exarust
reproduces, weighted by bytes, over every component the retail game actually
runs.

- **Denominator:** live bytes only. Components marked `not-applicable`
  (dormant editor code, unused RTL, linker thunks) are left out entirely.
- **Weight:** bytes from `functions.json`, not function count and not lines of
  Rust. A 300-byte rule and a 6 KB pipeline are not equal.
- **Credit:** one behaviour is credited once. The combat-rule sub-score is a
  view over code already counted with its host owner; it never adds credit to
  the headline.
- **Engine credit:** Bevy, wgpu, rapier or std may be credited only for
  behaviour Exarust actually uses and that is equivalent (not merely similar).
  A different falloff curve or a different ordering is not equivalent.

## Status bands

| coverage | status | meaning |
|---:|---|---|
| 0–4 | `missing` | no counterpart, or too little to count. **Contributes 0.** |
| 5–79 | `partial` | some rules reproduced |
| 80–100 | `done` | "reimplemented" by the byte rule |

The band is a label, not a claim of completeness. A component can be at 81%
and still miss a user-visible feature; say which one in the evidence.

Ratings of 1–4% are a trap: they look like credit but contribute nothing. If
the work is real, find enough reproduced behaviour to reach 5%; if not, rate 0
and describe the partial work as context.

## Evidence format

Every rating carries:

1. paths that exist (the audit verifies this, including in the published page);
2. the names of the functions/rules that carry the behaviour;
3. the gap, stated plainly;
4. the weighting sentence: which original function ranges earned credit and
   which did not.

**Good**

```
coverage: 64
exarust:
  - crates/exa_ai/src/path.rs (A*, cost model, snap, line walk, shortcuts, end trim)
evidence: "The two-layer floor rule, A* with the 7/5 step costs, the
  nearest-walkable snap, the line walk, the point shortcut, the end trim and
  the shortcut budget are reproduced. The scene-wide countdown and the
  remaining grid-update path are not. Weighted against the original's 24
  functions, the reproduced ranges are ~20.8 KB of 32.5 KB."
```

**Bad**

- `"mostly done"` — no paths, no gap, no weight.
- `"ports the original's behaviour"` — which rules, which constants?
- a citation to a file that was renamed (the audit fails this).
- a percentage lifted from `docs/PLAN.md` without checking the code.

## Choosing a percentage

1. Enumerate the original's functions (or coherent rule groups) for the
   component and their byte sizes.
2. Mark each: reproduced / partly reproduced / absent.
3. Reproduced means **the same rule with the same constants and ordering**.
   A structurally different approximation that happens to look right is at
   most partial.
4. Partial credit is proportional to the rules covered, not to effort. Cap it
   when the input data differs (e.g. a missing upstream decode makes the
   output untrustworthy).
5. Sum the reproduced bytes, divide by the component's bytes, round to whole
   percent.

Worked example: a 32.5 KB pathfinder whose search, costs, snapping and three
shortcut rules are in, but whose incremental grid update and scene-wide
countdown are not, lands in the low 60s. The exact number matters less than the
list of included and excluded rules — the list is what a reviewer can refute.

## Section sanity

After rating components, check the section:

- If a section is under 1% but a crate lists it among its main sections, the
  table is stale. Rerate, or label the headline a floor.
- If a component over 50% appears in "largest missing", a status is wrong.
- If a section's prose says 29% while the headline says 27.3%, one of them is
  stale; regenerate rather than editing either by hand.

## Where the evidence comes from

- `context/re/component-map/sections/*.md` — section notes and placement
  rationale.
- `context/re/*.md` — the behaviour write-ups with addresses and open
  questions.
- The decompiled functions via the repo's REA/Ghidra setup
  (`docs/RE-TOOLING.md`, the `reverse-engineer-anything` skill).
- The code itself: read the module, do not trust its README.

When the original's behaviour is not decoded yet, the rating is at most
"partial" and the evidence says so. An unknown is not a reproduction.

## Failure modes

| symptom | cause | fix |
|---|---|---|
| page and images disagree | built at different times | regenerate both in one run |
| data ahead of sources | `data.json` hand-edited | move the rating into `ratings.json`/`fixes.json` |
| rating vanishes after a rebuild | source still says 0 | restore it in the source; the runner refuses silent reverts |
| dead file in evidence | refactor without re-check | update the path or drop the claim |
| section at 0% with real code | section never rerated | rerate, or publish a floor |
| 3% credit that never counts | sub-band rating | decide: 0, or ≥5 with real rules |
| number changes with no code change | input edited concurrently | re-run after the tree settles; record `dirty` |
