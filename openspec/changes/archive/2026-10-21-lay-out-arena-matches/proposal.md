# Proposal

## Why

A career match spawns its opponents and places its fighters by the
template's setup: one opponent across the spawn spot, a ring of team
matches, six fighters of a fray. Exarust only had a guessed ring for two
fighters, and play mode could only stage a duel.

## What Changes

- The fixed opponent and company places (2 m either side of the spawn
  spot, the opponent 5 cm up), the 3.5 m ring at a fraction of a turn with
  the engine's sine and cosine, the fray's six places.
- The eleven setups: who each spawns, at which level and gear tier (one or
  two lower for Valiance, Valour and Captain's wings; no gear for
  Pugilism), on which team, and where the company's entrants stand.
- The next round of a multi-round match: a fresh opponent while the
  fighter stands, else the kept opponent against the next reserve.
- The rounds a template sets (Elimination, Reserve) in the running match.
- Play mode `--arena-match <id>`: the player takes the company's first
  place and a duellist stands on each opponent place; the tick ends the
  rounds.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: match layouts, the next round, the running match's
  template.

## Impact

- New `exa_game::match_layout`; `arena_match::MatchRounds::for_template`;
  `exarust` play mode.
- Docs: `docs/formats/OTHER.md`, `docs/DEBUGGING.md`, `crates/exarust/README.md`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (the eleven setups, the ring
  placement, the placement templates, the spawner's arguments, the next
  round and the reserve pick). Notes: `context/re/arena-layout.md`.
- Out of scope: the body and kit generators the spawner calls, the beast,
  the company's other entrants in play mode, the floor snap and the
  next-round spawn in the running game, the opponents' teams in the AI.

## Phase

Phase 7, M7 "arena season": staging the matches.
