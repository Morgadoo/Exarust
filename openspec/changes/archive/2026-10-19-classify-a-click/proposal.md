# Proposal

## Why

Phase 4 draws the world cursor and steers the player. A click does not yet say what it found, whether the point is close enough to use, or whether the body must turn toward it first.

## What Changes

- A hinge that is not blocked is a door or lever. A fitting is a wheel or a crank. A dead character is a corpse and a living one a person.
- A loose prop lighter than 0.08 is ignored. A heavy prop is ignored when strength is under 0.9. A usable item on a prop stays usable. No body is nothing.
- A point farther than 70 cm is turned toward first. It is within reach at 245 cm of height and 300 cm away, or 300 cm high and 350 cm away for a door.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: what a click finds, and when the point is close enough.

## Impact

- `exa_game::pick`.
- `docs/PLAN.md`, `docs/PLAYER.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the cursor classification and the walk-to-use checks. No format field changes, so the corpus step is a skip.
- Out of scope: the drag spring, pouring and repair, and opening the container.
