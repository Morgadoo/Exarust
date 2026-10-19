# Proposal

## Why

Phase 7 ends an arena day. A hired trainer does not yet teach a combatant whose current technique the trainer knows.

## What Changes

- With a trainer hired, a night adds 1000 divided by one plus the sum of the six skill levels to the first skill whose level is under 5 and whose technique at that level the trainer knows.
- A sum that reaches a new multiple of 5000 snaps to that multiple.
- A merchant, a physician, and a company with no trainer gain nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the trainer's nightly lesson.

## Impact

- `exa_game::career`, the day the career already ends.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the lesson and the skill word it adds to. No format field changes, so the corpus step is a skip.
- Out of scope: item prices, emblems, the hire price, and copying a loadout when a level is gained.
