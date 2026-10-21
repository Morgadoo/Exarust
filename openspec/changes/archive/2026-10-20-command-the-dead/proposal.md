# Proposal

## Why

`TPwrCommandDead_C` applies a command-dead effect to a target and removes it, using marker `0x10010d`. It mirrors the loyalty power without the marked-target refusal. Exarust has loyalty but not command-dead.

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
- Evidence: the step (the `0x10010d` marker and the target's command-flag write) and the removal scan (kind `0x10010d` and owner, setting the effect's state 2 and clearing the owner's effect field). No new corpus files.
- Out of scope: the power's serialization and the visual.

## Phase

Phase 7, thaumaturgy. The command-dead power.
