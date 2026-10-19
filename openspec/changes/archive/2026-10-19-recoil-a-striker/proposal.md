# Proposal

## Why

A blocked striker still returns to its stance at once, and a wound does not slow the swing that landed it.

## What Changes

- A recoil sets strength and the stall rate to 0.5 and clears the released bit.
- A class word of 0x90 with the continuation bit, or with a clock that widens past 0.4 and a counter, stops there.
- Any other swing sets stalling, cleaving and recoiled, blend 40, and both arm stiffnesses to 0.2. The end truncates the widened drive times 0.1 times 400, then adds now and 200.
- A follow-through with that same skip writes nothing. Otherwise the scale is the widened gain times recoil times speed times 0.15, floored at 0.
- A body zone at or under the epsilon sets strength 0. A larger zone uses its square root times the scale. An edge uses one minus the zone times the scale plus the zone. Strength caps at 1.
- The stall is the widened one minus strength times 0.05. Flags gain stalling and cleaving. The end truncates the scale times 400 and adds now.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the striker's recoil and the wound's follow-through.

## Impact

- `exa_combat::swing`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the recoil and the follow-through after a wound. No new corpus files.
- Out of scope: the caller's choice of gain, injuries beyond damage, and practice mode.

## Phase

Phase 4, the first duel. The striker's recoil and follow-through.
