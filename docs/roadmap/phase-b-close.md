# Phase B — Close every open function

**Goal:** no ledger row is `missing`, `partial` or `uncalled`.

**Size:** known only when Phase A ends. The estimate today is 306 KB of
gap (12.2%). A keyword sort of the current gap texts suggests the classes
below; the ledger will replace these figures (task A8).

| class | rough share of the gap | what closes it |
|---|---|---|
| named missing or partial | 30% | a port |
| ported, no live caller | 14% | a wire-up, or a proof it is unreachable |
| engine-replaced | 9% | evidence that behaviour matches |
| unreachable | 3% | the caller proof |
| character creator | 3% | the other session (decision DEC-3) |
| no gap text | 39% | Phase A decides |

## Tasks

### B0. Open leads (before anything else)
Possible defects found on 2026-11-11. Each gets a small lane.

1. **Save slot kinds.** The original's name builder may map the save and
   the checkpoint the other way round from Exarust's slot list. Check
   against the assembly and against a copy of a retail save folder, read
   only. Fix before any other change to code that deletes saves.
2. **Level-exit save position.** The save written at a level change
   records the new level with the old level's player position.
3. **Control Mind end.** In free mode the frame still writes the duellist
   and the player body over newer simulation steps.
4. **Torrent in lockstep.** One sweep scene no longer hits the level since
   the bar's casts draw from the step's random streams. Confirm against the
   original which stream the player's cast uses.

5. **No save slot for a new game** (found by the modes-03 audit). The
   original creates a slot for both managers when a new story or career
   starts; Exarust creates one only for a scripted debug save, so the
   story's exit, revive and checkpoint saves write nothing until a slot is
   continued from the save screen. The arena's own saves (match begin,
   in-game quit, forfeit) are dropped because the save steps run only in
   story mode. The golem alarm state and the journal are not saved.

**Done when:** each is fixed with a test and a scene, or shown correct.

### B1. Wire the uncalled
For each `uncalled` row: find the original's caller; if Exarust has that
caller, wire it; if the caller is itself missing, the row joins B4; if the
original has no live caller either, the row becomes `unreachable`.

Cheapest class: the code exists and is tested. Each wire-up needs a scene
that shows it running.

**Done when:** no row is `uncalled`.

### B2. Reclassify the engine-replaced
For each family (GL program binders and state, the kd-tree builder thread,
the deferred-call ring, hook lists, threads and critical sections, the
shader compile path, window and context set-up):
1. name the replacement;
2. state what observable behaviour the original code produced (a picture,
   an order of events, a timing, a value);
3. show Exarust matches it (shot, trace or test);
4. close the rows as `engine`.

Where behaviour does not match, the row stays open and goes to B4.

**Done when:** every such family is closed or handed to B4.

### B3. Close the unreachable
For each candidate: the proof is one of no caller, a caller that is itself
unreachable, a flag no code writes, or an editor-only caller. A second
agent confirms each proof (as in A5).

**Done when:** every candidate is `unreachable` or back in B4.

### B4. Port what is missing
The real work. Lanes as in rounds 130–202, with these changes:

- one component family per lane, taken from the ledger in byte order;
- the lane starts from the ledger rows, not from a gap text;
- the report ends with ledger rows; a second agent rates it;
- every port has a scene that reaches it, not only a unit test.

Expected order, to be confirmed by A8:
1. rendering: renderer passes (opaque-phase scissor, particle pre-pass,
   hook lists), mesh and geometry activation, terrain composite draw;
2. UI: item views, form dispatch, controls, the remaining skin slots;
3. thaumaturgy and effects: per-power remainders, effect types;
4. world: scene nodes (reparent, hand-off, queued removal), grid
   environments, content streaming;
5. characters and items: morphing, import, item GUI, descriptions;
6. physics tail: body part remainders, rigid-body clone, dynamics helpers;
7. game modes: arena remainders, game main (quit autosave, level-load
   finish, practice filter);
8. platform: OS functions, Steam (decision DEC-2).

**Done when:** no row is `missing` or `partial`.

### B5. Kept deviations
Every difference kept on purpose becomes a `deviation` row pointing at a
decision. Known so far: the supersample jitter, the path planner's shared
stamp table (kept for determinism), platform items with no Linux meaning
(screen-saver refusal, work-area fit), acceleration structures replaced by
Rapier. Each needs the owner's decision (DEC-5).

**Done when:** every `deviation` row cites an approved decision.

### B6. Character creator
SG_CharacterGen, SG_Character_Create and TCharCreation belong to the
owner's other session. Their ledger rows are audited in Phase A like any
other; the ports stay with that session unless the owner says otherwise
(DEC-3).

## Working rules for Phase B

- Three porting lanes at a time, as now; GPU scenes go through the run
  queue (task D1.2) so grids do not run the card out of memory.
- Checks before a round is published are unchanged: format, lint, the whole
  test suite, spec validation, a play build, a scene in the game, the
  leak scan.
- A lockstep trace that changes must be explained in the round's notes.
- The last three round binaries are kept while lanes compare against them.

## Risks

| risk | answer |
|---|---|
| Wiring an uncalled routine changes behaviour that looked right | each wire-up needs a retail comparison where one exists |
| Ports in rendering cost frame rate | the rule stands: no visible loss of quality or frame rate; measure in the same grid |
| The long tail of tiny functions is slow | batch them by family; one lane closes a family, not a function |
| Determinism breaks again | the determinism check runs on every round |
