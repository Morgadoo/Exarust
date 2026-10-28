# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: the hub
draws and launches a match, but the match's end screen, its next round,
the return to the hub, the hub's own bodies and several company-window
values were not reproduced, and the launch dropped the event's rank, gear
tier, entry fee and drawn weapon variants.

## What Changes

- `exa_game::arena_session`: the round result's state and transition
  screen, the end screen's Continue (main menu, next round or a
  tournament's next bracket fight, settle and return with the saved
  manager placement), the forfeit guard, the next round's bodies, the
  dead-body pass before the end screen, the return's health write-back,
  booking and tournament-day checks, the saved lineup's participant list
  and the hub arrival fixups.
- The running match keeps its transition screen and exposes Continue and
  forfeit; the hub's Begin charges the entry fee, warns once on the board
  when the manager is entered, lets the first unhealthy specialist leave,
  takes the player's body from the manager's pool slot, and the restart
  carries the rank, gear tier and one weapon variant per opponent
  (`--arena-level`, `--arena-tier`, `--arena-weapons`).
- Hub placements for the shopkeeper and the specialists, the roster's
  health gauge, the fighter panel's values, the outfit quality markers, the
  loadout preview modes and the shop's first-stock guard as tested rules.
- Review fixes against the assembly: the next round keeps the opponent who
  won when the reserve comes in; the dead-body pass compares in double;
  the immediate Continue follows a switch the game never sets; arena4's
  view amount stays 0.6; the prize history refuses all eight slots; the
  entry fee floors the purse at 0 instead of refusing; the shopkeeper does
  not copy its condition word; the panel health is the double product; the
  gauge geometry follows the GUI unit; outfits carry eight quality markers;
  the preview's camera scales with the unit.

## Evidence

The retail round result, end screen Continue, forfeit, next round,
dead-body pass, return to hub, start-after-save, hub arrival, shopkeeper
and specialist placement, match start, board Begin, weekly tournament,
fighter panel, roster gauge, quality marker and preview routines, read from
the assembly of Exanima.exe 0.9.5 (the double compare after CVTSS2SD, the
eight-slot prize loop, the floor-at-zero purse subtraction, the view
amount table's 0x3f19999a, the gauge's unit arithmetic).

## Out of scope

- Creating and tearing down the scene bodies (generic, hire-list and
  Beast opponents, the company entrant, the hub's shopkeeper and
  specialists); the match's end screen input and its return to the hub in
  the running game (the launch restarts the process, so the career does not
  survive into the match); the shop's first stock in the running hub (no
  `ShopWorld` resource); the original's form constructors.

## Phase

Phase 7 (game modes and content), the arena: the match's end and the hub's
launch.
