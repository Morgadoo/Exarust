## Why

C10 gave the clips the original's clocks (0.6 × base standing, 0.75 off
balance, 0.9 getting up, the swing rate attacking, the gait's rate
walking), its loops and its starts; what is left of the original's clock
block and sampler is still Exarust's own. The posture block's rate is one
table: its last line resets the rate to the base while the clip playing is
the body's stance, whatever the balance, so a dazed body in its Defence
stance plays it at 0.6 a tick where Exarust plays 0.75; the kick runs at
twice the base; an attack's clip off balance runs at the falls branch's
rate, not its swing's; a shocked body's clock stands still. The sampler is
one loop in `exa_anim`, so the changes that fill it (the gait overlay, the
femur posture, the IK chain, the torso twist, the head look, the foot
pitch) have nowhere to go. Bodies without a body tick still run a 60
frames a second clock the original never had, and the trace shows neither
the frames a tick added, nor the progress, nor the end test.

Phase 3 (Bodies: skeleton, physics, control), milestone M3 "standing
man", the master plan's MS1 "original timing". This is change M14 of the
master plan (`context/re/master-plan.md`, lane B, B1), on main's C10, C11,
C12, M22 and M0's stages, what the plan lists as left of it.

## What Changes

- `exa_game::limp`: the posture block's rate (`clip_rate`: 1.25 × base
  off balance, (1.5 − 0.5 × encumbrance) × base getting up, 2 × base
  kicking at the tone a standing body keeps, 1 × base otherwise, and
  1 × base while the clip is the stance) and the clock block's advance
  (`clip_frames`: 0.6 × rate; tone × rate × 0.6 getting up; the swing's
  rate attacking, or the posture rate off balance; the gait clock
  walking; none while the condition's bit 0x80 holds the clock still);
  `Pace` carries the stance and the still bit; `clip_tick` returns the
  hold for the sampler instead of sampling.
- `exa_game::stages::sampler`: the original's per-part pipeline, one
  function a step (the keys blended, the overlay, the hold and the kept
  targets, the femur posture, the clip chain, the IK chain, the twist,
  the head look, the foot pitch), identity where the change has not
  landed (M16, C13, C19, M8); the carry stage reads the sampled parts.
- `exa_anim`: `Animator::bracket` (which writes the progress first),
  `blend_keys`, `hold_toward`, `keep`; the tick keeps its `rate` and
  `ended`; `Animator::progress` (Exarust's first-to-last measure) and
  `speed` go; the pose-only `sample` takes the same bracket (a looping
  clip blends from its last key to its first instead of clamping);
  bodies without a body tick tick by the body clock through an
  accumulator (`owed`), 0.6 × their `base` a tick; `FRAMES_PER_SECOND`
  goes, `TICKS_PER_SECOND` (83) names the clock; `Clip::loops`.
- `exa_combat::Condition::clip_stands_still` (bit 0x80).
- The body trace adds `rate`, `progress` and `clip_end`.

Evidence:
- `exe.asm` of the body tick's posture block (the off-balance branch's
  jump to the block's end, the kick and clinch branches, the combat idle,
  weapon switch, power and other-clip branches, the stance reset at the
  end) and clock block (the get-up's tone gate, attacks, the 0x80 bit, the
  end test), with every constant read from `Exanima.exe` through REA
  (1.0, 2.0, 1.25, 1.5, 0.5, 0.6, 0.85, 0.5, 0.95, 0.02);
  `context/re/master-animation.md` §3, §7.1 and the M14 findings.
- The installed `humanoid.rsq`: Walk's keys 10…80 (first and last 19.9°
  apart), the stances' first and last keys, Get up's first key 20 and Get
  Up 2's 15.
- Unit tests pinned to the numbers; the headless trace test across a loop
  wrap (the GPU had no memory for a light run).

Out of scope:
- the gait overlay, the locomotion clock's stride modulation, start key
  and plant events (M16); the femur posture, the foot pitch and the IK
  chains (C13, C19); the torso twist and the head look (M8); the clinch
  (no clinches in 0.9.5); the weak branch's rate kept from last tick
  (Exarust reads the base; documented); a kick input (none in 0.9.5).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `motion-playback`: the clip rate by posture, the clocks (the stance,
  the kick, attacks off balance, shocked), the pose-only sampler's
  bracket, the sampler pipeline, the clocks of bodies without a tick,
  the clock quantities exposed.
- `physical-body`: the trace's `rate`, `progress` and `clip_end`.
