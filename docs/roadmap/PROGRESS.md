# Progress

The tracking file for the [roadmap](README.md). Update it at the end of
every round (see "How to update" at the bottom).

**Status: Phase A running (batches of 4 packages), with B0 in parallel.
Decisions DEC-1 to DEC-4 taken as recommended; DEC-5 and DEC-6 open.**

## Headline

| measure | start (r186, 11-10) | now (r218, 11-15) | target |
|---|---|---|---|
| Estimate: reimplemented, by bytes | 81.9% | 82.3% | replaced by the ledger |
| Estimate: engine-provided | 5.1% | 5.1% | — |
| Estimate: by function count | 73.4% | 73.8% | — |
| Combat rules | 90.6% | 90.6% | 100% |
| **Ledger: bytes in a final state** | — | **51.23%** | **100%** |
| Ledger: functions audited | — | 6,626 of 6,478 | 6,478 |
| Ledger: audit packages accepted | — | 90 of 90 | 90 |
| Live rows with scene or retail evidence | — | 289 of 2,643 (10%) | 100% |
| Retail references | — | 8 | about 50 |
| Scenario matrix rows passing | — | 0 (matrix not built) | all |
| Tests passing | 8,489 | 8,637 | all |
| Specs valid | 87 | 87 | all |

The ledger starts low because nothing has been confirmed function by
function yet; see the [README](README.md).

## Phases

| phase | state | next step |
|---|---|---|
| A — measure every function | done | — |
| B — close every open function | doing | B0 leads; then B1 (1,018 uncalled) |
| C — behaviour | doing | C2 needs a scratch retail profile (owner); C4 scenes for test-only rows |
| D — release | not started | D1 infrastructure |

## Tasks

State: `todo`, `doing`, `done`, `blocked`.

### Phase A ([detail](phase-a-ledger.md))

| id | task | state | note |
|---|---|---|---|
| A1 | Ledger skeleton and packages | done | 8,783 rows, 90 packages (2026-11-12) |
| A2 | Reachability, second pass | done | walk through unit init, class tables, data blocks, callbacks and unwind handlers; dead candidates 740 → 49 functions (8 KB); the editor is wholly unreached; exceptions all explained (RTL libraries, never-constructed base classes) |
| A3 | Audit brief and three pilot packages | done | 3 pilots in 7-9 min and 136k-256k tokens each; brief and tools improved; Phase A now estimated at 25-30M tokens |
| A4 | Audit packages | done | 90 of 90 audited (5 redone after rejection, 2 settled by a second review) |
| A5 | Review of each package | done | 90 accepted |
| A6 | Coverage computed from the ledger | done | published 2026-11-13: ledger 48.3% closed (37.8% live, 8.8% engine, 1.7% no code), 39.6% partial, 12.1% open |
| A7 | Check the excluded code | done | 102 of 103 reached not-applicable rows confirmed; the RTL random group reopened as ported code; 36 game-class engine rows checked (33 engine, 1 partial, 2 unreachable) |
| A8 | Phase B work list | done | `phase-b-worklist.md`: 1,038 uncalled (291 KB), 1,548 partial (988 KB), 74 missing (16 KB) |

### Phase B ([detail](phase-b-close.md))

| id | task | state | note |
|---|---|---|---|
| B0.1 | Save slot kinds | done | inverted: a fresh saves screen continued from the checkpoint, and the portal reset deleted the checkpoint while the save stayed unlisted; fixed (round 203). Creator-side names left to the other session |
| B0.2 | Level-exit save position | done | round 204; arrival marker shown in game 2026-11-15 |
| B0.3 | Control Mind end frame writes | done | round 206; game run 2026-11-15: no undone write logged |
| B0.4 | Torrent stream in lockstep | done | the original draws from the two process-wide generators, which the step streams stand for; the sweep scene needs retuning, not the code |
| B0.5 | New game creates no save slot; arena saves dropped | done | round 205 (tests); the `--story-session` debug flag skips the real start, so a flag for a real New Game start is owed for a game check |
| B0.6 | Spirit Form adopt/home and other frame-sent writes in free mode | done | round 208; three bridge-ownership hazards left (item cursor drain race, one-step-stale copy-in values, the frame-side combat heading write) |
| B1 | Wire the uncalled | doing | rounds 209-219: character data, body dynamics, Mind powers, journal and manual, shader templates, arena board, morphing, saves, line skin; uncalled 1,014 → 862 rows; lanes running: skill screens, item GUI |
| B2 | Reclassify the engine-replaced | doing | round 213 closed the shader templates (DEC-8); lane running: runtime, data and core rows (with B3) |
| B3 | Close the unreachable | todo | |
| B4 | Port what is missing | doing | rounds 216, 218: refraction and subsurface, held trees and the respawn portal; lane running: TMotileAI |
| B5 | Kept deviations | blocked | decision DEC-5 |
| B6 | Character creator | blocked | decision DEC-3 |

### Phase C ([detail](phase-c-fidelity.md))

| id | task | state | note |
|---|---|---|---|
| C1 | Comparison tools (seed, capture, diff, evidence) | done | round 207 tools; the ledger's evidence column: live rows by retail 11, scene 246, test 1,938, code reading 319 |
| C2 | Retail reference set | todo | 8 of about 50 |
| C3 | Scenario matrix | todo | sweep has 80 scenes |
| C4 | Unit-test-only ports | todo | first entries: ui-01 picker title band and hovered label (two_ink_band, glow_text), promoted by review from code reading |
| C5 | Playthroughs | todo | |
| C6 | Free mode clean | todo | one conflict known |
| C7 | Retail saves | todo | unblocked by B0.1 |

### Phase D ([detail](phase-d-release.md))

| id | task | state | note |
|---|---|---|---|
| D1 | Working infrastructure | doing | D1.2 done: `context/tools/gpu_grid.sh` runs one grid at a time with a free-memory check (after the GPU loss); gpu_grid.sh refuses the 615 driver after ten GPU drops |
| D2 | Performance | todo | |
| D3 | Platform | todo | Steam: decision DEC-2 |
| D4 | Regression suite | todo | |
| D5 | Packaging | todo | |
| D6 | Final review | todo | |

## Audit by section

Packages are listed one by one in [`ledger-summary.md`](ledger-summary.md).

| section | packages | accepted | open functions | open bytes |
|---|---|---|---|---|
| Physics, bodies & cloth | 7 | 7 | 184 | 187,026 |
| UI | 12 | 12 | 435 | 169,427 |
| Rendering & GPU | 10 | 10 | 297 | 151,277 |
| AI & behaviour | 8 | 8 | 190 | 120,216 |
| Thaumaturgy & effects | 9 | 9 | 168 | 119,006 |
| Characters, motion & animation | 8 | 8 | 237 | 107,529 |
| Game modes, arena & session | 7 | 7 | 228 | 93,831 |
| World, levels & terrain | 5 | 5 | 172 | 88,138 |
| Items, materials & crafting | 5 | 5 | 184 | 86,068 |
| Data, assets & I/O | 6 | 6 | 161 | 45,896 |
| Runtime & platform | 5 | 5 | 81 | 24,381 |
| Core, math & threading | 3 | 3 | 98 | 18,405 |
| Script, dialogue & story | 2 | 2 | 31 | 7,304 |
| Audio | 1 | 1 | 7 | 3,295 |
| Combat, weapons & damage | 1 | 1 | 5 | 2,201 |
| Other, tools & editor | 1 | 1 | 0 | 0 |
| **Total** | **90** | **90** | **2,478** | **1,224,000** |

## Open decisions

See [`DECISIONS.md`](DECISIONS.md): DEC-1 definition of 100%, DEC-2 Steam, DEC-3
character creator, DEC-4 start and budget of Phase A, DEC-5 kept deviations,
DEC-6 composite terrain, DEC-7 approximate float instructions, DEC-9 Calm's write past the contacts.

## Log

| date | round | what |
|---|---|---|
| 2026-11-15 | 215–218 | Morph transfer on the original's plane barycentrics, the heal sound, the came-loose hook (wiremorph); refraction for glass and gems, subsurface for candles, soap and ceramics, the fast-loop shadows (refract, B4); running powers saved with the player and rebuilt on load, the GUI reset on level change (wiresave); the held-tree trio, Spirit Form's disable, the respawn portal F542 (holdtree, B4). First rounds on driver 620, no GPU drop. Ledger 50.7% → 51.2%. |
| 2026-11-15 | — | The GPU fell off the bus again (Xid 79, the tenth since 2026-10-02) after the network card move. The drops began right after the 2026-10-01 update to kernel 7.2 and driver 615 and happen at idle as well as under load, so the driver is the leading suspect; the owner updated to kernel 7.3.3 and driver 620.70.06. `gpu_grid.sh` now refuses runs on a 615 driver. The five running lanes were restarted from their worktrees. |
| 2026-11-15 | 210–214 | Five B1 lanes: body settling, freeze and held-item park (wiredyn); the Sense Minds sight, the awakening glow and Loyalty's relation (wiremind); the journal's text box and the manual's item view, and a form defect that kept every record release from its callback (wireui); the shipped shader variants checked against the WESL, closing the template machinery as engine-replaced under DEC-8 and adding the translucent and fast-light programs (wireshader); the arena match board and session ends (wirearena). Ledger 48.5% → 50.7%. |
| 2026-11-15 | 208–209 | Spirit Form adopts and returns on the simulation step, and frame-sent events wait for the last step packet (B0.6); character data wiring (B1). Game run: both mind powers return the player, no undone writes; ledger 48.5%. |
| 2026-11-15 | 206 | GPU back after the PC fix (the network card had split the GPU's PCIe lanes; it fell off the bus 9 times since October; a stress test passed). Game smoke runs owed for rounds 204-206 done, no GPU errors. Phase B lanes restarted: spiritstep (B0.6), wiremind, wirechardata, wiredyn (B1). |
| 2026-11-13 | — | Phase A audits complete: all 90 packages accepted; coverage now published from the ledger (48.3% closed beside the 82.3% estimate); Phase B work list generated; 540 findings. Rounds 204-206 (arrival, new-game slot and saves, Control Mind on the step) pass their checks and wait for the GPU. |
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
