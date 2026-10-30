# Proposal

## Why

The C3 lane (the AI senses) found that the original runs the AI target pass
(sense, admit and classify every character and contact) for one brain a
tick, the one that has waited longest, while Exarust ran it, and the
perception inside it, for every brain every tick. That is a fidelity gap (a
brain notices, engages and leaves combat on its own turns only) and a CPU
cost: exanima06's 47 statues each perceived every character every frame.

Phase 6, AI. The target pass's schedule; no docs/PLAN.md edit in this
change (the main session owns it).

Evidence (REA, Ghidra on `Exanima.exe`): the game loop's step (its pre-tick
hook list, the step counter it advances before the world tick, and the hook
list dispatcher, which keeps a hook that answers true), the AI rotation hook
(it rotates both the route planner's turn and the target pass's turn), the
AI tick (the presence state machine before the integration) and the contact
integration's present-state branch (the turn test, the stamp of the step
counter, the call of the target pass, and the unsigned wait compare that
names the next brain).

## What Changes

- `exa_ai::pass_turn` (new): the `PassTurn` resource (a
  `route::PlanTurn` on its own counter), its rotation system
  `rotate_pass_turn` (once per body tick, before the perception), the claim
  each present brain makes, and `forget`.
- `exa_ai::brain::perceive_all`: only the brain whose turn it is, and only
  while it is in the scene, perceives; the others keep what their last pass
  perceived.
- `exa_ai::brain::ai_tick`: the integration's present flag follows the
  brain's presence; a present brain claims the turn; the brain on its turn
  runs `run_pass`, the others `Npc::tick_between_passes` (morale drift and
  integration only); the duellist's entry and leave act only on a tick
  whose pass ran. A debug line names the brain on each turn.
- `exa_ai::brain::forget_removed`: a removed brain leaves the queue.
- `AiPlugin` inserts `PassTurn` and runs the rotation in the brain chain.

## Out of scope

- The AI tick rate: Exarust runs the AI once a frame on the latest body
  tick, the original every body tick (a separate change). The turn counts AI
  ticks, which keeps the order of the turns.
- C3's split of the integration around the target pass (aim, pass, drift),
  which is on its own branch; this change keeps main's order on the pass's
  tick and needs nothing from it.
- The knocked-out brain's perception skip in `perceive_all` (C3's branch):
  here such a brain never claims the turn, so it is only reached when it
  falls between its nomination and its tick.
- The route planner's turn, already ported (`route::PlanTurn`).
- exanima06's direct start (see the design: not a regression).

## Impact

- `crates/exa_ai/src/pass_turn.rs` (new), `brain.rs`, `lib.rs`,
  `crates/exa_ai/README.md`.
- Spec `npc-perception`: the perception and the AI tick run the target
  pass on the brain's turn; a new requirement for the turn.
