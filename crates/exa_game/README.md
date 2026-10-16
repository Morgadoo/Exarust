# `exa_game`

Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already runs the character ticks of physical bodies (`limp`: each body's
bearing, balance against the level, clip and targets, the player's last),
Exarust's falls and get-ups from the measured balance (`balance`), the
player (`player`), the held torch (`torch`), the per-tick body trace
(`trace`), and the debug tools ([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)):
the body overlay and joint readout (`body_overlay`), the joint trace
(`joint_trace`) and the simulation controls (`sim_control`).

## Scope

- App states (boot → menu → loading → in-game).
- Arena: companies, fighters, matches, rewards; Story: levels Underworld 0..8; Practice/sandbox.
- Save-game format of our own (original saves are not a compatibility goal).
