# Phase A — Measure every function

**Goal:** no function is `unaudited`. Coverage is computed from the ledger.

**Starting point (2026-11-12):** 8,783 functions. 2,305 are closed by their
component (not-applicable, engine-provided, or rated 100%). 6,478 functions
(2.37 MB) in 384 components are `unaudited`. They are cut into 90 audit
packages of at most 110 functions and 45 KB each.

The work is uneven: 484 functions of 1,000 bytes or more hold 997 KB (42%
of the open bytes); 2,277 functions under 100 bytes hold 99 KB.

## Tasks

### A1. Ledger skeleton — done
`build_ledger.py` builds one row per function from the function list, the
splits and renames of the component map, and the cross-reference export.
`make_packages.py` cuts the open rows into 90 packages.
**Done when:** the ledger builds with no unplaced function. Done 2026-11-12.

### A2. Reachability, second pass — done 2026-11-12
Result: roots are the entry point, the TLS callback and the 131 units'
init and final procedures; classes, data blocks, stored callbacks and
unwind handlers are walked. Dead candidates fell from 740 functions to 49
(8 KB). The check passes when the two runtime libraries are left out:
7 of 2,241 rows of game components rated 95% or more are unreached (base
class stubs and editor-only code), 5 not-applicable rows are reached from
live code (hooks that return early). The builder now needs
`scan_pe.py`'s output.

The first pass walks from the entry point and from every function a data
table points at. It is wrong in both directions: it reaches unused runtime
classes (every class table is a root), and it misses calls through
registered callbacks, so units known to run show as unreached.

1. Roots: the entry point, the unit start-up table, and the tables of
   classes that some reached function constructs. A class nobody constructs
   is not a root.
2. Add the edges the export lacks: virtual calls resolved through the class
   tables (`classes.json` has every slot), and callbacks registered with the
   hook lists, the timer table, the task queue and the console.
3. Check the pass against known facts: every function of a component rated
   95% or more must be reached; the dormant editor must not be.
4. Write the result into the ledger's `reached` column.

**Output:** a list of dead candidates each audit lane starts from.
**Done when:** the check in step 3 passes with under 1% exceptions, each
explained.

### A3. Audit brief and pilot — done 2026-11-12
Results: physics-02 (26 functions, 7 min, 136k tokens), render-01 (41,
8 min, 256k), ui-01 (73, 9 min, 190k). The brief gained the tools
`check_resolved.py`, `show_fn.py`, `callers.py`, `rust_live.py`, a reading
list, rules for large functions, adaptations, shader logic and undriven
arms. Corrected cost of the remaining packages: about 200k tokens per
audit plus about 100k per review, so 25 to 30 million tokens in all.

1. The brief is in `context/tools/rounds/audit-brief.md`.
2. Run three pilot packages of different kinds: one physics (large
   routines), one UI (many small ones), one rendering (much engine-replaced
   code).
3. Measure: functions per hour, tokens per package, share of rows the
   review rejects.
4. Fix the brief and the package size from what the pilot shows.

**Done when:** the three pilots are reviewed and accepted, and the cost of
the remaining 87 packages is known.

### A4. Audit packages (90)
One read-only lane per package. For each function the lane:
1. reads the original (REA, checked against the assembly);
2. finds the Rust item, or establishes there is none;
3. finds the live caller in Exarust, or the original's callers;
4. writes one row: state, Rust item, proof.

No code changes in an audit lane. A defect it notices is recorded as a
`partial` or `missing` row with the detail in the proof, and goes to
Phase B.

Eight lanes at a time. Order: sections with the largest open bytes first
(rendering, UI, thaumaturgy, world, characters, items), so the Phase B work
list is useful early.

**Done when:** every package has a row for each of its functions.

### A5. Review of each package
A second agent, which did not write the package:
1. re-checks every row closed without code (`engine`, `unreachable`,
   `deviation`), because those raise coverage with no new code;
2. re-checks a random 15% of the `live` rows against the assembly;
3. rejects the package if more than 5% of the sampled rows are wrong; the
   lane then redoes it. When the rejection rests on a single wrong row in a
   small sample, and the reviewer has already corrected that row, a second
   reviewer may instead check every row the first did not; the package is
   accepted if those hold (added 2026-11-13).

**Done when:** every package is accepted. `PROGRESS.md` lists each one.

### A6. Coverage from the ledger
1. `recalc_coverage.py` reads the ledger: reimplemented = `live` bytes,
   engine = `engine` bytes, closed without code = `unreachable` and
   `deviation` bytes, open = the rest.
2. The component map page shows the four shares per component.
3. For one round the page shows both numbers, then the estimate is dropped.
4. The round workflow changes: a lane's report must end with ledger rows,
   and the rating comes from them.

**Done when:** a published round takes its number from the ledger.

### A7. Check the excluded code
1,343 functions (167 KB) are not-applicable, and 933 (138 KB) are
engine-provided, by component-level decisions. With the A2 pass:
1. confirm no game code path reaches a not-applicable function; move any
   that is reached back into the count;
2. sample 10% of the engine-provided rows and confirm the replacement and
   the matching behaviour.

**Done when:** both checks are written up; any function moved is in a
package.

### A8. Phase B work list
From the finished ledger: every open row, grouped by component family and
ordered by bytes, with its class (uncalled, partial, missing).

**Done when:** `phase-b-close.md` carries the real sizes.

## Cost and time

Yesterday's lanes each read 35 to 119 routines in 33 to 71 minutes and used
225,000 to 440,000 tokens, while also writing code. On that basis, as an
estimate to be corrected by the A3 pilot:

- 90 audit lanes plus 90 lighter reviews: 30 to 45 million tokens;
- eight at a time: about 12 batches, two to three working days.

## Risks

| risk | answer |
|---|---|
| An audit closes rows too easily | A5 re-checks every row closed without code |
| The ledger number looks like a regression | both numbers are published until A6 |
| Lanes disagree on what `live` means | the brief defines it; the pilot tests it |
| Usage limits (about 20 agents at once hit them) | eight audit lanes at a time |
| The function list is itself wrong (803 owners are low-confidence) | a lane that finds a misplaced function reports it; the split goes into `fixes.json` |
