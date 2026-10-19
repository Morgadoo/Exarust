# Proposal

## Why

The in-game menu's fourth command is what starts an arena match. While the mode is arena it removes an unhealthy specialist, asks for a save, marks the hub busy, and moves the session into a transition. That branch is not in the career yet.

## What Changes

- Accept the command only on the in-game page, only for command 4, and only in arena.
- Run the specialist leave, then ask for a save that does not abort on low health.
- Mark the hub busy, clear the world click hook, and set the session phase to transitioning.
- Leave the company and those flags unchanged for story, practice, another command, or another page.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the in-game menu command that begins an arena match.

## Impact

- `exa_game::menu`, calling `Company::leave_before_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the menu branch and the three stores that arm the match. No new corpus files.
- Out of scope: closing windows, writing a save package, and playing the transition.

## Phase

Phase 7, arena career. One menu command, not the match load.
