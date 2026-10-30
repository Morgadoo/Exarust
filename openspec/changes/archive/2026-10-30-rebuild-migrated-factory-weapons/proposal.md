## Why

The object database unit's remaining routines (round 37): the dormant editor's look roll, the container child and counted-list accessors, the migration's factory-weapon step and the real paint-seed draw of the oldest body layout were missing or stubbed. The review found the factory step left out the condition refresh the original always runs after it and did not write the factory's stored choices back into the body.

Phase 3 (world and objects: the object database) in docs/PLAN.md.

## What Changes

- The editor's look roll: seven colour families, the trim's one-in-five row, the light byte, the finish, two cleared bytes, in the original's draw order.
- The container body's child rows accessor and the counted-list accessor with its signed count test.
- Migrating a factory weapon (model named `@…`) hands the name and body to a caller-supplied factory pass, then always scales the stat bytes by the weapon's own condition and rescales its model's value word; the item side's pass writes the first-seed fix with its initial choices, or a stored random roll, and the recipe's stats into the body.
- The oldest body layout's paint seed from two twister draws.

## Evidence

The object database pack's assembly: the look roll's generator steps, tables (families 0, 4, 8, 14, 19, 25, 28; trim pairs; count 7) and the 6.0 constant; the accessors' `+0x18` and signed `>= 1` test; the migration's `@` compare followed by the factory call and the unconditional weapon condition refresh; the factory's first-seed fix and choice writes; the 63-bit random draw modulo `0xFFFFFFFF`.

## Out of scope

Running the migration in the level loader or saves (no caller yet); the editor itself; the factory's colour argument mapping (the item factory component's).
