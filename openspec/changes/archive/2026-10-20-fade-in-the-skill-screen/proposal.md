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
- Evidence: the 2,955-byte skill-screen tick's bit-1 block: bit 1 of the state word, the second deadline against the clock, the state byte 4, the page against 2, the close slot, the sound field, the delay global and the sound call. No new corpus files.
- Out of scope: the draw calls after the fade.

## Phase

Phase 4, UI. The skill screen's fade-in.
