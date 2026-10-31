# Design

## The original's schedule

The physics thread's step (one per 83 Hz step, however many a frame) runs,
in order: a sync of the simulation, a global step callback, the pre-tick
hook list (when the simulation has it on; the AI's rotation hook is in it,
handing over the target pass's and the route planner's turns), then it
adds one to the step counter and the step's milliseconds to the game clock,
publishes both, runs the simulation's own callback and pre-step list, and
steps every sector, stamping each with the counter. A sector's step ticks
its characters first, from the last registered to the first (the player,
registered first, ticks last), then integrates and solves the bodies.

A character's body tick, after its daze bookkeeping, chest velocities and
jolt test, calls its behaviour's tick through the behaviour's virtual
table: the player's behaviour for the player, the AI's tick for an NPC.
That tick runs the morale, the presence step, the contact integration (and
the target pass on the brain's turn), the state function (idle, combat,
flee) and the head look, and registers the footprint. The body tick then
takes the falls and get-ups, the health bookkeeping, the movement decision,
the body turn, the locomotion, the clip sampling, the balance and the feet.

So the AI runs once per body tick for every character in a sector, and
everything it times (the target pass's turn, the planner's turn, its
millisecond timers on the game clock) counts body ticks.

## Exarust

The brain chain was an `Update` chain, once per frame, guarded to run at
most once per body tick: at more than one tick a frame it skipped ticks.
It now runs in its own schedule (`exa_ai::BrainChain`, single-threaded:
its systems run one after the other anyway, and in the fixed step's
multi-threaded executor each of the seventeen paid a dispatch and its sync
points waited for unrelated systems), run by one exclusive system in the
`exa_ai::BrainTick` set of `FixedUpdate`:

- in `exa_physics::body::TickBehaviour` (new), the first of the fixed
  step's body sets: before the character controllers
  (`exa_physics::CharacterControllers`, new: their register, sweep and
  world step), the pre-step, the grip build and the character ticks, so
  the velocities, attack requests and switches the AI writes are taken by
  this tick's controllers, grips and character stages, as the player's
  input (`FixedPreUpdate`) already is;
- after `presence::check_presence` (the AI tick's presence step comes
  before its integration) and the daze's stance (`StanceTick`: the body
  tick's daze comes before its behaviour call), before the posture
  (`PostureTick`: the falls come after it).

The fixed step's systems that share data with the brains' tick were
unordered against it (Bevy's ambiguity report, `--schedule-ambiguities`,
listed 48 such pairs): the grips and the hands' words, the held weapons'
sync, the mechanisms' registration, the kinesis pick and the thrown
objects run after `TickBehaviour` now, the entering cast reset before the
brains' tick, and the game's parts inside it are ordered among
themselves; the report lists none for the brains' tick.

In the chain, in order: the roles and the player's rating, the rotation, the
perception, the AI tick, the encounter scripts, the duellists' power
answers, the combat state (`DuelSystems`), the power cast, the flight, the
entering turn, the idle walk, then the follow and search and the sentry.
The removal forget stays in `Update`: it reads the frame's removed
components, which a fixed step that does not run in a frame would miss.

The per-tick guards (`BrainClock`, the rotation's last tick, the
duellists' tick count) stay: in the fixed step each run is a new body
tick, so they pass once a run, and they still protect a caller that runs
the chain between ticks.

`pass_turn` sets its counter to the body ticks run so far plus one, the
step counter the original's brains read after the hook list ran and the
counter advanced (`BodyTicks` counts at the end of the solver, so during
the brains' tick it is one behind). The route planners keep their own
counters, which now count ticks too.

## Differences left

- The original ticks each brain inside its body's tick, between the other
  characters' ticks; Exarust runs every brain before every character tick,
  as it runs the player's input. A brain reads the others' state from the
  end of the last tick, where the original's later brains see the earlier
  characters' whole tick.
- The brains read `GlobalTransform`, which propagates once a frame
  (`PostUpdate`): within one frame's ticks a character's position as the AI
  sees it does not move. Propagating the character roots each tick would
  also change what frame systems read between the fixed loop and the
  propagation; left for a separate change.
- The footprints are kept in `FixedPreUpdate`, before the brains, so their
  route gate reads the last tick's route.

## Cost

Measured with the perf log and the profiler's new `brains` line, four
games at once on a loaded host (artifacts README). Moved as a plain chain
into the fixed step, the brains' tick took 0.43 ms a run in the arena (one
brain) and 0.85-1.40 ms a run in exanima06: most of it was the fixed
step's multi-threaded executor (a dispatch per system, sync points
waiting for unrelated systems). Run on one thread in its own schedule it
takes 0.04-0.08 ms a run in the arena and 0.28 ms in exanima06, and
gathering the fighters' places once a tick instead of once per brain in the
AI tick brought exanima06 to 0.17 ms a run (0.67 ms a frame at four
ticks). In paired runs (this branch and main side by side) Fixed moved by
+0.7 and -1.9 ms of about 130 ms and Update fell by 0.5-0.6 ms (the chain
left it): no measurable cost.

## Open questions

None for the schedule; the order inside the character list (above) is the
one gap, answered already by REA (the sector step's character loop).
