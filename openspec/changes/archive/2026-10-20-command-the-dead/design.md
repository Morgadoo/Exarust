# Design

## Behaviour

The power keeps a state and a target id. Event 2 on state 1 returns 2 (the framework removes it); on any other state it sets state 2. An applying state looks the target up: a missing target moves to state 2 and returns 0; a valid target applies the effect marker `0x10010d`, sets the target's `+0xd40->+0x5a` byte, returns to state 0 and returns 0. A removing state scans the target's effects for kind `0x10010d` owned by this power, sets that effect's state to 2, clears the target's `+0x590` and returns 2.

## Model

`CommandDeadState { state, target }`, `step_command_dead(dead, event, target_valid) -> CommandDeadStep { state, result, applied, removed, marker }`.

## Open questions

The serialization and the visual are separate.

## Docs

`SG_Thaum_Mind` is **13%**. The headline stays 14.4% and 5.7%.
