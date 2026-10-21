# Design

## Behaviour

The power keeps a state and a target id. Event 2 on state 1 returns 2 immediately (the framework removes it); on any other state it sets the state to 2. An applying state (1) looks the target up: a missing target or one carrying `0x800` moves to state 2 and returns 0; a valid target applies the effect marker `0x10010a`, sets the target's command flag, returns to state 0 and returns 0. A removing state (2) removes the target and returns 2.

## Model

`LoyaltyState { state, target }`, `step_loyalty(loyalty, event, target_valid, target_marked) -> LoyaltyStep { state, result, applied, removed, marker }`.

## Open questions

The controlled-list allocation and the visual are separate.

## Docs

`SG_Thaum_Mind` is **12%**. The headline stays 14.4% and 5.7%.
