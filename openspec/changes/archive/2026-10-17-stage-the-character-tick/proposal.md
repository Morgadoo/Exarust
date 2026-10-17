## Why

Exarust's character tick is one function, `limp::tick_characters`, that
does every living body's whole tick in a single loop, and its solver is one
function, `body::solve`, whose loops every later physics change must edit.
The master plan puts the work of seven lanes into that tick and that
solver: clocks and samplers, locomotion, attacks, condition, feet and pins,
pairs between bodies, props' mechanisms, world links, cloth. They would all
edit the same lines. The original's tick has a fixed order of stages: the
sector step runs every character's body tick, the player's last, then the
solver (integration, 20 iterations each starting with the pair pass,
contacts dropped, sim objects, body–body detection, sim-object contacts,
body–level detection, 8 friction passes with the pairs' first, post-step),
and each body tick runs its parts in a fixed order. `exa_audio` is one file
that four lanes will add sounds to.

Phase 3 (Bodies), milestone M3 "standing man". This is change M0 of the
master plan (lane A, the enabler every lane builds on); it depends on
nothing and changes no behaviour.

## What Changes

- The fixed step's tick gets named sets in the original's order: a
  pre-step, the character ticks, the solver, a post-step.
- The character ticks run a schedule of nine named stages (upkeep,
  readiness, anchor, behaviour, posture, attack, movement, sampler,
  carry) once per living character, in the original's order (the player
  last), each character's whole tick before the next one's. The ticked
  character is a resource while the stages run.
- `tick_characters` is split into a driver and one module per stage:
  posture (what the body does), movement (gait floor, the controller's
  step, balance and ground), sampler (clock, capture, sample), carry
  (targets on the frame, lift, the controller's follow). The rules they
  call stay in `limp`.
- The solver calls registered solver stages at each of the original's
  stages: after integration, the pair pass at the start of each
  iteration, links between bodies after each iteration's body links,
  after the iterations, the sim objects, body–body detection, sim-object
  contacts, the pair friction of each friction pass, after the post-step.
  Nothing is registered yet.
- `exa_audio` is split into the bank (with one sound-set picker every
  event uses), music, hits and footsteps.
- A debug helper, `exarust --fixed-frames`: every frame advances game time
  by exactly 1/60 s, so a run repeats tick for tick and traces compare
  between builds.
- No behaviour changes: the same ticks give the same bodies and the same
  traces.

Evidence: REA and `exe.asm` of `Exanima.exe` 0.9.5, the sector step
(sim objects, the character list from its last entry down, 20 iterations
with the pair pass first, the pair list dropped, detection in the 3×3
cells, body–level detection, 8 friction passes with the pair friction
first, the post-step) and the body tick's calls in address order (burning,
Break Tangle, the daze capture, the behaviour update, waking, the swing
routine, the focus tick, the locomotion, the sampler, balance, the feet
routine); `context/re/body-tick-order.md` §1, `master-body.md` §10,
`master-movement.md` §2, `master-animation.md` §3.4, `master-combat.md`
§16. Runs of Exarust: `--trace` of `--shove 60 play arena` against the
build before this change.

Out of scope:
- moving the stance, falls, attack stamp, player input and AI (which run
  ahead of the character ticks or once a frame) into their stages: each
  moves with the change that rewrites it;
- measuring the balance after the sampler as the original does (Exarust
  keeps measuring it before, as today);
- pairs between bodies (C17), sim objects (M37, M49), world links (M33),
  doors, levers and plates (M12, M35), the post-step's hits (C21);
- the original's sound-set probing (01 to 16, no repeat) and its audio
  environment.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: the tick's sets, the character tick's stages and the
  solver's stages, in the original's order.
- `audio`: sound sets and the shared picker.

## Impact

- `exa_physics` (`body.rs`: `BodyPlugin`, `CharacterTicks`, `BodySystems`,
  `solve`, `step_bodies`; new `body_stages.rs`: `BodyPreStep`,
  `BodyPostStep`, `CharacterTick`, `CharacterStage`, `TickedCharacter`,
  `run_character_ticks`, `SolverStage`, `SolverStages`, `solve_with`)
- `exa_game` (new `stages/`: `CharacterTickPlugin`, the driver and the four
  stage modules; `limp.rs` keeps the rules)
- `exa_audio` (`bank`, `music`, `hits`, `footsteps`; `SoundSet`)
- `exarust` (`--fixed-frames`)
- `docs/formats/RSQ.md` (Physical body), `docs/ARCHITECTURE.md`,
  `docs/DEBUGGING.md`, `docs/PLAN.md`, crate READMEs of `exa_physics`,
  `exa_game`, `exa_audio`
