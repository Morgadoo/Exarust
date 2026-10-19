# Proposal

## Why

A shown value is counted and not passed on. A cleared mark does not read a world byte. Raising the session mark to 1 is not in.

## What Changes

- A shown value is passed with the two platform words. The indirect target is not called.
- An open world sets the side from its byte before the bank select. A lock keeps the inverted side. A force stores 0. A differing byte selects that bank and is written back.
- Raising the mark to 1 stores the flag 1 and the code 0, clears three notice bytes, installs index 0 without copying a bank, clears a hold byte, sets a view bit, and clears a view code.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the platform draw, the world side, and the session mark raised to 1.

## Impact

- `exa_game::menu`, from `MatchGate::begin_match` and `MatchGate::set_session_aside`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the platform call's three words, the world byte, and the call that passes 1. No new corpus files.
- Out of scope: the indirect call's target, and callback registration.

## Phase

Phase 7, arena career. The draw arguments and the two mark paths, not the whole cursor.
