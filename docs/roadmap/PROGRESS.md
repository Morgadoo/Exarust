# Progress

The tracking file for the [roadmap](README.md). Update it at the end of
every round (see "How to update" at the bottom).

**GPU: the RTX 3090 dropped off the host on 2026-11-13 after an out-of-memory error (`nvidia-smi` finds no device); it needs a reboot. Audits continue (CPU only); rounds needing a game smoke wait.**

**Status: Phase A running (batches of 4 packages), with B0 in parallel.
Decisions DEC-1 to DEC-4 taken as recommended; DEC-5 and DEC-6 open.**

## Headline

| measure | start (r186, 11-10) | now (r202, 11-11) | target |
|---|---|---|---|
| Estimate: reimplemented, by bytes | 81.9% | 82.3% | replaced by the ledger |
| Estimate: engine-provided | 5.1% | 5.1% | — |
| Estimate: by function count | 73.4% | 73.8% | — |
| Combat rules | 90.6% | 90.6% | 100% |
| **Ledger: bytes in a final state** | — | **48.41%** | **100%** |
| Ledger: functions audited | — | 6,478 of 6,478 | 6,478 |
| Ledger: audit packages accepted | — | 89 of 90 | 90 |
| Live rows with scene or retail evidence | — | not measured | 100% |
| Retail references | — | 8 | about 50 |
| Scenario matrix rows passing | — | 0 (matrix not built) | all |
| Tests passing | 8,489 | 8,536 | all |
| Specs valid | 87 | 87 | all |

The ledger starts low because nothing has been confirmed function by
function yet; see the [README](README.md).

## Phases

| phase | state | next step |
|---|---|---|
| A — measure every function | doing | A2, A3 pilot |
| B — close every open function | doing | B0 open leads |
| C — behaviour | not started | C1 tools |
| D — release | not started | D1 infrastructure |

## Tasks

State: `todo`, `doing`, `done`, `blocked`.

### Phase A ([detail](phase-a-ledger.md))

| id | task | state | note |
|---|---|---|---|
| A1 | Ledger skeleton and packages | done | 8,783 rows, 90 packages (2026-11-12) |
| A2 | Reachability, second pass | done | walk through unit init, class tables, data blocks, callbacks and unwind handlers; dead candidates 740 → 49 functions (8 KB); the editor is wholly unreached; exceptions all explained (RTL libraries, never-constructed base classes) |
| A3 | Audit brief and three pilot packages | done | 3 pilots in 7-9 min and 136k-256k tokens each; brief and tools improved; Phase A now estimated at 25-30M tokens |
| A4 | Audit packages | doing | 89 of 90 accepted; core-03 being redone. Five packages were rejected and redone (characters-01, characters-03, items-03, physics-05, core-03) and two were settled by a second review (physics-01, ai-04) |
| A5 | Review of each package | doing | 2 rejections so far, both for live rows whose state or input nothing live supplies |
| A6 | Coverage computed from the ledger | doing | |
| A7 | Check the excluded code | doing | 1,343 not-applicable, 933 engine rows |
| A8 | Phase B work list | todo | |

### Phase B ([detail](phase-b-close.md))

| id | task | state | note |
|---|---|---|---|
| B0.1 | Save slot kinds | done | inverted: a fresh saves screen continued from the checkpoint, and the portal reset deleted the checkpoint while the save stayed unlisted; fixed (round 203). Creator-side names left to the other session |
| B0.2 | Level-exit save position | doing | the save was right; the arrival was wrong (the simulation kept the old level's coordinates). Fixed on a lane; round 204 waits for the GPU smoke |
| B0.3 | Control Mind end frame writes | todo | |
| B0.4 | Torrent stream in lockstep | todo | |
| B0.5 | New game creates no save slot; arena saves dropped | doing | all four claims real and fixed on a lane (slot at session begin, arena saves, quit save, alarm and journal); round 205 stacks on 204 and waits for the GPU |
| B1 | Wire the uncalled | todo | size after A8 |
| B2 | Reclassify the engine-replaced | todo | |
| B3 | Close the unreachable | todo | |
| B4 | Port what is missing | todo | |
| B5 | Kept deviations | blocked | decision DEC-5 |
| B6 | Character creator | blocked | decision DEC-3 |

### Phase C ([detail](phase-c-fidelity.md))

| id | task | state | note |
|---|---|---|---|
| C1 | Comparison tools (seed, capture, diff, evidence) | todo | |
| C2 | Retail reference set | todo | 8 of about 50 |
| C3 | Scenario matrix | todo | sweep has 80 scenes |
| C4 | Unit-test-only ports | todo | first entries: ui-01 picker title band and hovered label (two_ink_band, glow_text), promoted by review from code reading |
| C5 | Playthroughs | todo | |
| C6 | Free mode clean | todo | one conflict known |
| C7 | Retail saves | todo | unblocked by B0.1 |

### Phase D ([detail](phase-d-release.md))

| id | task | state | note |
|---|---|---|---|
| D1 | Working infrastructure | doing | D1.2 done: `context/tools/gpu_grid.sh` runs one grid at a time with a free-memory check (after the GPU loss) |
| D2 | Performance | todo | |
| D3 | Platform | todo | Steam: decision DEC-2 |
| D4 | Regression suite | todo | |
| D5 | Packaging | todo | |
| D6 | Final review | todo | |

## Audit by section

Packages are listed one by one in [`ledger-summary.md`](ledger-summary.md).

| section | packages | accepted | open functions | open bytes |
|---|---|---|---|---|
| Physics, bodies & cloth | 7 | 7 | 194 | 188,931 |
| UI | 12 | 12 | 469 | 185,518 |
| Rendering & GPU | 10 | 10 | 343 | 174,256 |
| Thaumaturgy & effects | 9 | 9 | 206 | 129,720 |
| AI & behaviour | 8 | 8 | 190 | 120,216 |
| Characters, motion & animation | 8 | 8 | 262 | 116,341 |
| Game modes, arena & session | 7 | 7 | 253 | 105,681 |
| World, levels & terrain | 5 | 5 | 172 | 88,138 |
| Items, materials & crafting | 5 | 5 | 186 | 86,750 |
| Data, assets & I/O | 6 | 6 | 162 | 46,004 |
| Runtime & platform | 5 | 5 | 81 | 24,381 |
| Core, math & threading | 3 | 2 | 95 | 16,750 |
| Script, dialogue & story | 2 | 2 | 35 | 7,554 |
| Audio | 1 | 1 | 7 | 3,295 |
| Combat, weapons & damage | 1 | 1 | 5 | 2,201 |
| Other, tools & editor | 1 | 1 | 0 | 0 |
| **Total** | **90** | **89** | **2,660** | **1,295,736** |

## Open decisions

See [`DECISIONS.md`](DECISIONS.md): DEC-1 definition of 100%, DEC-2 Steam, DEC-3
character creator, DEC-4 start and budget of Phase A, DEC-5 kept deviations,
DEC-6 composite terrain, DEC-7 approximate float instructions.

## Log

| date | round | what |
|---|---|---|
| 2026-11-13 | — | UI, rendering, thaumaturgy and world audited. Two packages rejected and redone. The RTX 3090 dropped off the host after an out-of-memory error; `gpu_grid.sh` now queues GPU runs. Round 204 (arrival placement, B0.2) passes its checks and waits for the GPU. |
| 2026-11-12 | 203 | Rendering and UI fully audited: 26 packages accepted, ledger 20.7%, 151 findings. Round 203: the save slot kinds fixed (B0.1). |
| 2026-11-12 | — | 9 packages accepted, ledger 12.3%. The audits found 38 defects so far (`findings.md`), among them blank terrain blocks drawn and shadowing, glow link rings scrolling backwards, no refraction, the journal buttons unwired. |
| 2026-11-12 | — | A2 done (reachability through class tables and callbacks: 49 dead candidates left). Pilots accepted on review: physics-02, render-01, ui-01. Batch runner for the other 87 packages running. Ledger 6.9%. |
| 2026-11-12 | — | Phase A started: pilots physics-02 and render-01 audited. Both show the component ratings overstate: TVXGIManager is rated 97% but 58% of its bytes are open at function level; TMotileDynamics 97% with 88% of the package's bytes partial (mostly the two biggest routines, not re-audited block by block). Retail addresses removed from a code comment and five archived proposals. |
| 2026-11-12 | — | Roadmap written. Ledger skeleton built (8,783 functions); 90 audit packages cut; first reachability pass. Project still paused. |
| 2026-11-11 | 187–202 | 16 rounds: NPC concern and flight, wraith smoke, terrain holes, race fixes, parking, timer table, domain wheel, portal reset. Estimate 81.9% → 82.3%. Paused at the owner's request. |

## How to update

At the end of every round:

1. Run `python3 context/tools/ledger/build_ledger.py`. It rewrites
   `ledger-summary.md` and fails on a malformed audit row.
2. Copy the new headline numbers into the table above.
3. Set the state of the tasks that moved, and the accepted count of the
   sections that gained a package.
4. Add one line to the log: date, round, what changed, in plain words.
5. A new decision, or one the owner answered, goes into `DECISIONS.md`.
6. Commit this file with the round's coverage commit.

A task is `done` only when its "Done when" line in the phase file holds.
