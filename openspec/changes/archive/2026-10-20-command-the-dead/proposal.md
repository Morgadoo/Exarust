# Proposal

## Why

`TPwrCommandDead_C` (`0x1001e2770`) applies a command-dead effect to a target and removes it, using marker `0x10010d`. It mirrors the loyalty power without the marked-target refusal. Exarust has loyalty but not command-dead.

## What Changes

- Event 2 on an applying command-dead returns 2; otherwise it moves to state 2.
- An applying command-dead moves a missing target to state 2; a valid target applies marker `0x10010d`, raises the target's flag and returns to state 0.
- A removing command-dead removes the marked effect, clears the target's flag and returns 2.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the command-dead power.

## Impact

- `exa_game::mind`, from `CommandDeadState`, `CommandDeadStep`, `COMMAND_DEAD_MARKER` and `step_command_dead`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **13%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001e2770` (the step with the `0x10010d` marker and the target `+0xd40->+0x5a` write) and `0x1001e26c0` (the removal scan for kind `0x10010d` and owner, setting the effect's state 2 and clearing `+0x590`). No new corpus files.
- Out of scope: the serialization at `0x1001e2650` and the visual.

## Phase

Phase 7, thaumaturgy. The command-dead power.
