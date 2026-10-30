# Design

## The original's schedule

The game loop's step runs, in order: the pre-tick hook list, then it adds
one to the step counter and the step's milliseconds to the game clock,
publishes both, and runs the world tick (which ticks every AI). One hook in
that list, registered when the AI manager is set up, rotates two turns: the
route planner's (already ported as `route::PlanTurn`) and the target pass's.
Each rotation makes the brain the last tick named the current one, clears
the named brain and clears the longest wait. The hook answers true, so the
dispatcher keeps it.

Inside an AI's tick, after the presence step and the out-of-fight exit, the
contact integration takes the aim point, clears the worst hostility, steps
the soothe damping, and then, only when the brain's presence state is
present (1):

- refreshes the nearby list;
- if the brain is the current one, stamps the step counter as its last pass
  and runs the target pass (which clears the contacts' per-pass words, calls
  the sense for each visible character and each contact, admits and
  classifies them, enters or leaves combat, and walks the refresh cursor);
- otherwise takes its wait as the step counter less its last pass, an
  unsigned 32-bit difference, and, when the wait is strictly greater than
  the longest so far this tick, makes itself the next brain with that wait.

Then the per-contact drift runs for every brain (with a separate decay for
a brain that is not present).

So the pick counts ticks, not elapsed time; a brain never had a pass when
its stamp is still 0 (the object is zero-filled), and has then waited the
whole counter; ties keep the brain the AI list reached first; the current
brain does not compete on its own tick, so a lone brain has a pass every
other tick and `n` brains one every `n` ticks. A brain knocked out, not
present, or removed on its tick simply loses it; nobody runs the pass that
tick, and the others' waits go on.

## Exarust

`PassTurn` wraps `route::PlanTurn`, which already has the same claim (stamp
on the turn, strictly-longer wait otherwise) and rotation, on its own
counter. `rotate_pass_turn` steps the counter and rotates once per body tick
before `perceive_all`. Exarust's perception is its own system ahead of the
AI tick, so it is gated on the turn too: the original senses inside the
target pass.

The counter counts AI ticks. Exarust runs the brain chain once per frame on
the latest body tick (`BrainClock`), not every body tick; every brain stamps
and compares the same counter, so the turns come in the same order. When the
AI tick rate is ported, the counter can become the body tick count.

`Integration::active` (the present branch) was never set; it now follows
`Presence::moves`, so a dormant brain takes the other decay, does not wait
and is never named.

On a tick without its pass, a brain runs `Npc::tick_between_passes`: the
morale drift and `contacts::integrate`, which drifts the concern toward the
concern target the last perception left (the original keeps those words
between passes too). The pass's report stays, so the duellist's entry and
leave are gated on the tick whose pass ran (the original enters and leaves
combat inside the pass).

Without a `PassTurn` resource (unit tests of one brain) every present brain
runs its pass every tick, as before.

## exanima06's start

C3 reported that exanima06 starts the player at [3, 0, 3]. Runs of
the round-44 binary, a round-43 lane binary and a binary of 2026-10-29,
each `play exanima06` with no `--at`, all log "player starts at [3, 0, 3]":
the level has no fixed start in `exa_world::start::player_start`, so the
viewer falls back to the grid's stand-in spawn (the open, floored cell
nearest the grid centre). The original only enters exanima06 through a
level transition, which places the player at the arrival's marker. So it is
no regression, and the golem runs keep `--at 10.5,0,65`, as every lane since
the golem wake has.

## Open questions

- Whether any other class shares the stamp's slot (two writers outside the
  integration use the same offset in other objects; read as unrelated).
  Answer: REA on those routines' classes, if a later trace needs it.
- The AI tick rate (once a frame against every body tick). Answer: a
  separate change; the counter is ready to switch.

No docs/formats page is involved.
