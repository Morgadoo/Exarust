# Proposal

## Why

docs/PLAN.md, Phase 2 (static world) and Phase 5 (visual and audio
fidelity), wiring backlog S4: the arena progression's weather and time of
day. The outdoor preset's draw and the arenas' weather ambience were ported,
but a level load drew its day twice from two clock reads (once for the
visibility rules while models spawn, once for the lighting), so a load could
light fires for one day and place the sun of another. The seed's shape and
the time-of-day driving had not been checked against the career code.

## What Changes

- `exa_world::environment`: a load takes its outdoor preset seed once
  (`clock_seed`, now the original's counter word: the low word of the
  100 ns counter with its bytes reversed, `counter_seed`) and both passes
  draw from it. The lighting pass logs the draw (`scene preset: … outdoor
  draw from seed …: time of day, daylight, rain target, cloudiness, fog
  distance, wind, view amount`).
- Two debug aids: `EXA_TIME_SPEED=<days a second>` sets the scene
  environment's time speed, which the weather tick already honours (the
  shipped game never writes it), and logs the driven day every 120 frames;
  `EXA_TIME_OF_DAY=<t>` re-places the sun or moon after the draw.
- `exa_game::arena_match::outdoor_bed` names its second input the daylight
  factor it is (the field after the time of day), and the arena table notes
  that nothing of the career reaches the draw.

## Evidence

REA (Ghidra) and the instructions of `Exanima.exe` 0.9.5:

- The match start rolls the arena from the reversed counter word, then calls
  the map load with a second, fresh counter word; the map load hands the
  outdoor draw `(view flag 1, the arena's night byte, that word)`, and
  indoor arenas the indoor preset. No rank, day or match count is passed.
- The counter routine returns the performance counter's low word with its
  four bytes reversed; the draw replaces a zero seed with it.
- The draw's night branch reads the time of day at the field the sun
  placement stores the time in; the map load's night-bed compare reads the
  next field (the daylight factor) with `COMISS` against 0.5.
- The weather tick adds `dt × speed` to the time only when the speed field
  is above zero. No instruction in the image stores to that field (every
  store at that offset belongs to other classes), so it keeps the
  constructor's zero: a match keeps the time its draw gave.
- At night, with the view flag, the draw writes `0.5·(cloud − 2) + (time −
  0.7)` into the view's post-processor amount (the same field the indoor
  arenas set to 0.3 / 0.6); the view creates its post processor only when
  that amount is above the double 0.01.

## Out of scope

- The view's post-processor amount and what the post processor does with
  it (not traced); the scene environment persisting across loads (Exarust
  builds a new one per load, so the night amount reads constructor values).
- The map load's push of the wind into every scene character when the wind
  is above 0.1 (not traced).
- Re-running the ambience or the fires as a driven time crosses their
  thresholds (the shipped game never drives the time).
