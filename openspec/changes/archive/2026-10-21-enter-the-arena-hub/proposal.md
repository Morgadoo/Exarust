# Proposal

## Why

The arena hub is where a career is run between matches. Entering it
draws one of two tracks and where it starts; a click there opens the shop,
the match board, the manage window or a merchant's stock; a manager who
dies there ends the session. Exarust played the first hub track from its
start and had none of the hub's hooks.

## What Changes

- The hub's music draw (two tracks, two thirds and one third; four start
  places) and its alternative tracks.
- The hub's click hook: the two object names with the engine's
  sixteen-byte compare, the side value that tells the two machines apart,
  the merchant's stock, and the gates (busy, arena mode, a confirmation).
- The hub's session hook: a dead manager ends the session and wipes the
  saves under 0.01 health.
- Play mode: the hub draws its track and plays it at its volume from the
  drawn place (`exa_audio::LevelMusicStart`).

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the hub.

## Impact

- New `exa_game::arena_hub`; `exa_audio::LevelMusicStart`; play mode in
  `exarust`.
- Docs: `crates/exarust/README.md`, `docs/formats/OTHER.md`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (the hub entry, the hub
  session hook, the hub click hook, the name compare, the start offset
  setter). Notes: `context/re/arena-hub.md`.
- Out of scope: the shop, board and manage windows the clicks open, the
  hub's camera framing and the manager's placement, the shopkeeper's and
  specialists' figures.

## Phase

Phase 7, M7 "arena season": the hub.
