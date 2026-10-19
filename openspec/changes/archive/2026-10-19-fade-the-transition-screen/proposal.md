# Proposal

## Why

The transition screen draws a centred "TO BE CONTINUED" when its page is 2, with the skin's colour and its own alpha. Exarust does not model it.

## What Changes

- Page 2 sets blend mode 0x203, binds the colour and draws the fixed text centred on the screen width and the glyph height.
- Any other page draws nothing but still calls the base draw.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the transition screen.

## Impact

- `exa_ui::menu`, from `draw_transition_fade`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 260-byte transition method, its fixed string at 0x100314420 and its two font fields. No new corpus files.
- Out of scope: the base draw body.

## Phase

Phase 7, the interface. The fade text.
