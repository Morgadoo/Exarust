## Why

Roadmap task B4 (lane `replace`, docs/roadmap/phase-b-close.md): the
living placement of a character body at a matrix (TMotileDynamics) was
ported only as the spawn's arithmetic. Its other callers moved a placed
body rigidly: the arena's kept bodies between rounds stood where they
stood (a Transform insert on the model), the golem's pose reset and the
revive's roaming reset shifted the parts, and the surviving player's
level placement translated the body. None of them sampled the clip
afresh, reset the body's words or anchored the feet as the original
does.

## Evidence

REA (Exanima.exe), read in the assembly:

- The placement copies the frame, resets the twist frame to the
  identity, clears the walk-ahead, turning and stepping bytes and the
  gait flag word, the thrust 0, the balance radius base 60, tone and
  energy 1, writes the standing position, step target and chest from the
  frame's origin, the look and heading words from its forward (the
  heading flattened, normalised, its yaw by the arc cosine), every
  part's three correction quaternions the identity, runs the living
  sampler once with no time and its snap flag, zeroes the carry, then
  puts every part at the pose lowered by the pelvis's posed height on
  the frame at rest, each foot contact's anchor, step target, previous
  anchor and smoothed ground at its posed contact on the frame less the
  clip root plus the sole less 8 cm, each pin 5 cm under its posed
  point, the feet centre the anchors' mean with their states cleared,
  the chest point the pins' mean with no velocity, and calls the
  behaviour's heading snap. It does not wake the body.
- Its callers write the frame's height before the call, over the
  world's origin: the arena's kept-body reset its stand-height word,
  the golem pose reset its pelvis-target word (after the pose-only
  sampler), the hub entry that word plus 5, the save landing the probed
  floor plus that word plus 5; the revive reset keeps the pelvis's own
  height over the moved ground point.
- The kept-body reset clears the current motion and the capture key
  around starting clip 3 with no blend, and a non-player's combat target.
- The next round places the kept opponent at the opponent template plus
  the spawn and the continuing fighter at the template the match set-up
  points at the company's spot, facing −z; the tournament round at the
  bracket's ring place.

## What Changes

- `Ragdoll::place_living` and `Ragdoll::placement_heights`
  (`exa_physics`), `Locomotor::placed` also clears the turn, the
  request's moving and free-walk bytes and its flag word.
- `exa_game::limp::LivingPlacement` with `FrameHeight` (given, stand,
  target plus cm), run by the limp plugin's fixed step: the controller
  moves to the frame, the model takes its turn, the held items are
  re-placed after it.
- Callers: the arena's kept-body reset (`SceneCommand::Reset` carries the
  round place; the rigid `Place` command is gone), the golem's pose
  reset, the revive pass's roaming reset, the surviving player's level
  placement (`exarust` viewer) and the hub entry's manager.

## Impact

Simulation: kept arena bodies, golems sent home, the player put outside an
alarm, revived roamers and every surviving player's level placement now
stand on a fresh pose with reset words. Determinism and grid scenes owed
(GPU on hold).
