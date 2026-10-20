# Proposal

## Why

After the opening fade, the skill draw's bit 1 runs the fade-in: a passed deadline sets the screen to state 4 and either shows nothing on page 2 or closes, while time left scales the sound by the remaining count over the delay. Exarust has the opening fade but not this half.

## What Changes

- With bit 1 clear nothing changes.
- With the deadline at or below the clock the state becomes 4; page 2 returns alpha 0, any other page closes.
- Otherwise the sound becomes `(deadline − clock) × sound / delay`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's fade-in.

## Impact

- `exa_ui::skill`, from `skill_screen_fade_in`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: no coverage change; part of the still-open 2,955-byte draw.
- Evidence: the 2,955-byte `0x100226520` bit-1 block: bit 1 of `0x158`, the deadline `0x164` against the clock, the state byte 4, the page `0x1c0` against 2, the close slot `0xc8`, the sound `0x17c`, the delay `0x1002ad5b0` and the `0x10002a350` call. No new corpus files.
- Out of scope: the draw calls after the fade.

## Phase

Phase 4, UI. The skill screen's fade-in.
