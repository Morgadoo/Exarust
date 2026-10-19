# Proposal

## Why

The stagger walk exists, but a collapsing body's dying tick never calls it.

## What Changes

- After the tone and the pin gain fall, the dying tick walks the stagger when no step is already running.
- The chest and its fast velocity are the measured ones, in centimetres for the stagger and metres for the walk.
- A chest recorded at the origin, then a metre along z, then there again, anchors one foot on the first tick. The second tick leaves that foot.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the dying tick calls the stagger walk.

## Impact

- `exa_combat::collapse::dying_tick`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the dying tick's stagger, then the footwork. No new corpus files.
- Out of scope: the dropped weapon, and injuries beyond damage.

## Phase

Phase 4, the first duel. The dying tick walks the stagger.
