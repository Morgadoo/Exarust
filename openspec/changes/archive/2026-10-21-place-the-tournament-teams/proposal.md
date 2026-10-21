# Proposal

## Why

A tournament event spawns its teams from the bracket's entry slots and
stands them on one ring, each team spread around its own sixth. Exarust
had an approximate ring for its starts but not the grouping, the
original's arithmetic, or which opponent each team brings.

## What Changes

- Group the entry slots into teams of `per_team` slots, the team id the
  first held id, members the positive ids, up to the event's team count.
- The radius `teams × per_team × 50 + 100`, at most 350; each team's spread
  and start; the bracket place with its double 0.1666666.
- A team's opponent: a generic level-5 fighter under team id 2, else the
  company's fighter; hostile with two teams, else a free for all.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: tournament teams.

## Impact

- `exa_game::match_layout::{bracket_layout, bracket_place, bracket_opponent}`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (the tournament event spawn and
  its ring placement). Notes: `context/re/arena-layout.md`.
- Out of scope: spawning the teams in the running game, the company
  fighter's generator, later rounds' re-dressing, the allies between team
  members in the AI.

## Phase

Phase 7, M7 "arena season": the weekly tournament's events.
