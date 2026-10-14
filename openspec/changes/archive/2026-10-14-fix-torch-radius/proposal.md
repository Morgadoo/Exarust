## Why

Torch-lit surfaces were about 1.4× darker than in the original, and the
torch's reach shorter. The original widens the small fire's light radius by
1.25 for sizes above 0.2, comparing in double precision. A torch's size is
stored as a 32-bit 0.2 (0.2000000030), which is above 0.2, so the original's
torches reach 838.5 cm, not 671 cm.

## What Changes

- The small fire template's widening test in double precision, so torches
  get the widened radius.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: fire light radius.

## Impact

- `exa_world::fire` and `docs/RENDERING.md`.
