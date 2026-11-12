# Phase C — Prove the game behaves like the original

**Goal:** every scenario in the matrix below passes against the retail
game, and every `live` family in the ledger has been seen running.

**Starting point (2026-11-12):**
- 8 retail references exist (`context/re/refs/`): the main menu, the
  character creation screen, the player's look, and notes on blood, combat
  and the torch arm.
- The feature sweep has 80 scenes in 22 batches. It checks that Exarust
  runs and logs the expected line. It does not compare with retail.
- Lockstep runs repeat exactly, so two Exarust builds can be compared row
  by row. Retail cannot be traced that way; it is compared by picture,
  by measured value and by timing.
- Outdoor levels draw the sun and weather from the clock, so two outdoor
  runs never match.
- Several ports of rounds 187–202 are proved by unit tests only (the
  parry end at death, the AI re-aim, the exit nudge's move, the arrow
  pause, the core-crystal test).

## Tasks

### C1. Comparison tools
1. **Debug seed** for the outdoor sun and weather (`--env-seed`), so
   outdoor scenes repeat and can be set to match a retail capture.
2. **Retail capture set-up**: the retail camera tool can read, pin and fly
   the camera and take a shot. Add a scripted list of poses per level so a
   capture session is one command.
3. **Picture diff** tool: same pose, both runtimes; reports changed
   pixels and regions, with a noise floor measured from two Exarust runs.
4. **Evidence column**: each live row records `test`, `scene` or `retail`.
   The summary shows the share at each level.

**Done when:** an outdoor level gives the same picture twice, and one
retail pose can be captured and compared with one command.

### C2. Retail reference set
About 50 references, taken once, kept in `context/re/refs/` (never
tracked). All captures use a scratch profile; nothing is created in or
deleted from the owner's saves.

| group | references |
|---|---|
| Levels | two poses in each story level and each arena level |
| Outdoor | each weather, day and night |
| Menus | title, settings pages, save list, end-of-session menu |
| Character screens | inventory, character view, loot, trade |
| Powers | the power screen at interface units 4 and 5; each power's effect |
| Combat | stances, a blocked blow, a wound, a fall, a death |
| Creatures | each race in view, including the wraith |
| Text | fonts, tooltips, item cards, dialogue |

**Done when:** each reference has a pose, a retail shot and the Exarust
shot beside it.

### C3. Scenario matrix
A table of scenarios, each with how it is checked and its result. It
extends the feature sweep; each row names the ledger families it covers.

| area | scenarios (examples) | checked by |
|---|---|---|
| Movement | walk, run, sprint, turn, backpedal, stairs, falls | retail timing of a fixed input; trace |
| Combat | each attack, parry, clash, wound, knockdown, death | retail video frame counts; trace |
| AI | notice, search, engage, flee, return, sleep, wake | scene log against retail observation |
| Powers | every power: cast, effect, end, failure | picture and timing |
| Items | take, equip, drop, use, containers, race limits | scene |
| World | doors, levers, portals, level change, streaming | scene; picture |
| Story | each level's scripts, dialogue, zones | scene log |
| Arena | career, hub, shop, matches, recruits | scene; save round trip |
| Saves | save, continue, checkpoint, death, retail saves | file compare |
| UI | every screen and dialog | picture |
| Rendering | materials, lights, shadows, fog, weather, effects | picture |
| Audio | music, ambience, reverb, footsteps, voices | log of what plays and when |

**Done when:** every `live` family in the ledger is covered by at least
one scenario, and every scenario has a result.

### C4. Unit-test-only ports
List every `live` row whose evidence is `test`. For each, build a scene
that reaches it, or record why none can (then it stays `test`, named).
Start with the ones from rounds 187–202.

**Done when:** the list is empty or every remaining entry has a reason.

### C5. Playthroughs
1. Story: from the first level through every level, with the scripts,
   portals and saves on the way.
2. Arena: a career from creation to the last tier.
3. Each is first played by script (recorded inputs), then by the owner.
4. Every stop, crash or wrong outcome becomes a ledger row or a defect.

**Done when:** both run to the end without a defect that stops play.

### C6. Free mode
The lockstep mode is the test bench; players use free mode. With the
write log on, the whole sweep must show no frame write that undoes a
simulation step. One is known (the Control Mind end, task B0.3).

**Done when:** the sweep is clean in free mode.

### C7. Retail saves
Exarust reads the original's save files. Check with copies of retail
saves, read only: a story save, an arena save, a checkpoint, an old
(pre-0.9.5) save. The slot-kind lead (B0.1) comes first.

**Done when:** each kind loads to the same place, state and inventory.

## Risks

| risk | answer |
|---|---|
| Retail cannot be driven deterministically | compare measured values and frame counts over repeated captures, not single runs |
| Physics "feel" is hard to measure | fixed-input tests: same key sequence, compare distance, time and pose at marks |
| Capturing retail takes the owner's time | scripted pose lists; one session per group |
| A mismatch has no obvious cause | each scenario names its ledger families, so a failure points at code |
