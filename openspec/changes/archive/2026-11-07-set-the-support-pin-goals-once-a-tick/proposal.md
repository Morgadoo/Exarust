## Why

PLAN Phase 5 (bodies). The TMotileDynamics gap lane audited the body tick
kernel (routine #49) block by block against the assembly. Its carry block
(after the character frame, before the balance routine) sets the support
pin records' targets once a tick; Exarust led and clamped the pins again in
every solver iteration, from the pin's moving point and the moving feet
centre, and its stance score read the posed pin points, not the led and
clamped goals.

## Evidence

REA (Exanima.exe), read in the assembly of the body tick's carry: for each
part with a pin record, the pelvis pin's target is the frame times its
local point, led while the step episode runs and the chest is not out of
balance, then clamped to 60·scale cm about the feet centre; each thorax
pin's target is the frame times its local point, shifted 5·scale cm along
the frame's forward axis without support, led, then clamped to
(60 − 60·x or 60 − 30·x)·0.6 (double)·scale cm, the clamp putting it at the
centre's height; the thorax targets' mean is stored (the frame's origin
without any). The thorax lead has two branches by the step kind: the plain
one writes the exertion value c·h·−0.004 (forward cosine c above 0), the
long (kind 2) one boosts the speed term under 2500 cm², takes the direction
m − (2 − m)·s·2 and writes (1 − m)·h·−0.002. Both leads multiply by a
class-set-up constant, 85 over the 83 Hz rate (its .data value 85.0 has no
writer), which Exarust had as 1. The solver's pin pass (the living limb
link iteration) pulls toward the stored target without leading or clamping.

## What Changes

- exa_physics: `Ragdoll::pin_goals` / `set_pin_goals` (body_pins.rs) set the
  goals, the thorax mean and the lead's exertion in `Ragdoll::aim` and
  `hold`; `pull_support_pins` pulls toward them; `clamp_goal`,
  `unsupported_shift`, `thorax_reach` (double 0.6, no floor);
  `steps::lead_pose` gains the long branch and the exertion, and both
  leads `LEAD_RATE` (85/83); the balance's stance score reads the kept mean.
- exa_game: the carry stage hands the lead's exertion to the locomotion
  state (`stages/carry.rs`).
- docs/re/motiledyn-map.md: routine #49's carry block.

## Impact

Every standing body's pins now pull toward goals fixed for the tick, so
the arena duel and exanima01 lockstep traces change from the first ticks
(see the lane's evidence README for the first differing tick and cause).
