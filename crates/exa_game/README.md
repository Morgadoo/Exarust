# `exa_game`

Game modes and session flow: Story, Arena (company management, matches, seasons), Practice; save games; ties all plugins together.

**Phase:** Phase 4–7 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already runs the character ticks of physical bodies (`limp`: each body's
bearing (with its walk's lowest ankle while it moves, and its get-up's
first quarter), balance against the level, clip (the capture of a start
from the body's physical turns, the original's clock rate for what the
body does, the pose it holds while dazed), targets on its character frame
and lift, the player's last; its controller is told to follow the frame;
a get-up's clip ending into the stance writes the condition word normal;
a collapsing body's bearing says so; a body asked to go limp keeps its
motion unless given a velocity),
Exarust's falls and get-ups from the measured balance (`balance`; a fall
stops the controller carrying and holding the body and keeps its tone; a
get-up turns the body, starts its clip from the whole lying body, lets
its frame re-anchor, writes the condition word getting up and ends in the
fighter's stance; falls start after the condition's posture stage), the
player (`player`: tank controls whose steering starts only the gaits, the
chase camera on the drawn pelvis, combat facing, its combat stance and the
original's attack request each behaviour tick (press, hold, release, the
double click and Alt) through the attack executor, with the clips of the
moveset its hands pick),
the world cursor's shape from the player's condition word (`cursor`), the health
bars' values and the low-health heartbeat with its sound (`hud`), the
held torch (`torch`, which fills its hand; its tests pin the "Torch"
clips' arm on the body model), the per-tick body trace
(`trace`, ending with the condition word), and the debug tools ([`docs/DEBUGGING.md`](../../docs/DEBUGGING.md)):
the body overlay and joint readout (`body_overlay`), the joint trace
(`joint_trace`) and the simulation controls (`sim_control`, with the
scripted pauses at a tick, a stretched link, the first blood jet, the
player's first landed blow or the first collapse).

## Scope

- App states (boot → menu → loading → in-game).
- Arena: companies, fighters, matches, rewards; Story: levels Underworld 0..8; Practice/sandbox.
- Save-game format of our own (original saves are not a compatibility goal).
