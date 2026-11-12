# Roadmap to 100%

How Exarust gets from the round-202 state to a complete reimplementation.
The analysis behind it is in
[`../reviews/2026-11-12-road-to-100.md`](../reviews/2026-11-12-road-to-100.md).

| file | what it holds |
|---|---|
| [`PROGRESS.md`](PROGRESS.md) | the tracking file: headline numbers, task status, log |
| [`phase-a-ledger.md`](phase-a-ledger.md) | Phase A: measure every function |
| [`phase-b-close.md`](phase-b-close.md) | Phase B: close every open function |
| [`phase-c-fidelity.md`](phase-c-fidelity.md) | Phase C: prove the game behaves like the original |
| [`phase-d-release.md`](phase-d-release.md) | Phase D: performance, platform, release |
| [`DECISIONS.md`](DECISIONS.md) | decisions taken and decisions waiting for the owner |
| [`ledger-summary.md`](ledger-summary.md) | generated totals of the function ledger |
| [`findings.md`](findings.md) | defects the audits found, waiting for Phase B |

## The two goals

**Code: every function accounted for.** The original has 8,783 functions.
The ledger gives each one a row, and each row ends in one final state.

| final state | what the row must carry |
|---|---|
| `live` | the Rust item that ports it, and how it was shown to run |
| `engine` | what in Bevy, wgpu or Rust std replaces it, and what was checked to match |
| `unreachable` | why no path in 0.9.5 runs it (no caller, a flag never written, an editor-only caller) |
| `not-applicable` | unused runtime library or the dormant editor |
| `deviation` | the decision in `DECISIONS.md` that keeps the difference |

| open state | meaning |
|---|---|
| `unaudited` | nobody has checked this function against Exarust yet |
| `missing` | reachable in the original, nothing in Exarust |
| `partial` | ported, with a branch or a write still missing |
| `uncalled` | ported, with no live caller |

Code coverage is 100% when no row is open.

**Behaviour: the game plays and looks like the original.** A `live` row
says code exists and runs. It does not say the result matches. Each live
row therefore carries its strongest evidence:

| evidence | meaning |
|---|---|
| `test` | a unit test with numbers worked from the assembly |
| `scene` | seen running in the game (a grid scene, a log line, a trace) |
| `retail` | compared with the retail game (picture, value or timing) |

Behaviour is complete when every scenario in the Phase C matrix passes
against retail.

## Two numbers during the transition

- **Estimate** (`docs/COMPONENT-MAP.md`): 82.3% at round 202. Reviewers'
  ratings per component. Kept until Phase A ends.
- **Ledger**: the share of counted bytes in a final state. It starts at
  5.6%, because only engine-provided components and fully rated ones are
  closed before any audit. It climbs as audit packages land, and replaces
  the estimate when Phase A ends.

The ledger number starting low is expected. It is not a loss of progress;
it is the part of the 82.3% that nobody has yet confirmed function by
function.

## Where things live

| path | tracked | holds |
|---|---|---|
| `docs/roadmap/` | yes | this plan, progress, decisions, generated totals |
| `context/re/ledger/ledger.tsv` | no | the ledger, with retail addresses |
| `context/re/ledger/resolved/<package>.tsv` | no | each audit package's rows |
| `context/re/ledger/packages.tsv` | no | which function is in which package |
| `context/tools/ledger/build_ledger.py` | no | builds the ledger and the summary |
| `context/tools/ledger/make_packages.py` | no | cuts the open rows into packages |
| `context/tools/rounds/audit-brief.md` | no | the brief an audit lane follows |

Retail addresses stay in `context/`, as the repository rules require. The
tracked files carry names and totals only.

## Order

1. Phase A (ledger and audits), with the three open leads in parallel.
2. Phase B, cheapest classes first.
3. Phase C starts as soon as Phase A shows which families lack a scene.
4. Phase D last, except for the items Phase C needs (the debug seed, the
   GPU run queue).
