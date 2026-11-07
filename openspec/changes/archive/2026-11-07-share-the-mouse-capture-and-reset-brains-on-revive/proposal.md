## Why

PLAN Phase 3 (the player) and the AI. Round 105 found that the original's
middle-button orbit and Cursor Look are one routine of the controls'
frame: it re-centres the pointer every frame while either is held and, on
release, warps the pointer to the screen point of the player's aim point.
Exarust centred once, turned the body's yaw directly and warped back to
the saved point; the orbit had no capture. The behaviour's ladder callback
was not verified, and the AI's caller of the "is the space free" query,
ported in round 123, was not wired.

## Evidence

REA (Exanima.exe):
- The capture routine polls the orbit and Cursor Look bindings (and the
  pad's Look and orbit bits). On the first held frame it fixes the kind
  (Cursor Look's when its key is down), saves the pointer, notes whether
  the GUI's hovered control is not the game view and, spectating, measures
  the look's direction and distance from the camera's look-at point on the
  plane 80 cm over it. Later frames: the orbit's drag message with the
  pointer's offset from the centre; Cursor Look sets the cursor follower's
  target to its target less 3.6 × Δx / width (wrapped within half a turn
  of the smoothed direction) and aligns the camera behind the drawn
  smoothed cursor direction. Every held frame the pointer is set to the
  client centre and the GUI pointer to the aim point's screen point (the
  saved point over the GUI). The release sends the drag's close message
  (the quarter-turn snap) for both kinds and warps, with the four-move
  hold, to the aim point's screen point or the saved point.
- The aim point: the drawn standing point at the ground plus 80 cm (the
  constant reads 80.0), plus the drawn smoothed cursor direction times the
  cursor's distance, projected through the view.
- Neither held, while the camera's align flag is set (set by the align
  routine, cleared by a drag or when the shown yaw is within 0.01 rad of
  the target, compared in double), the pointer's point on the aim plane
  under the frame before's view keeps its flat offset from the player.
- The behaviour's cursor routine skips the cursor's write while the mouse
  is captured and, while Cursor Look holds, writes the look's target
  vector as the raw cursor direction after the follower's tick.
- The behaviour tick asks the caster body's cast hook (installed only by
  the force powers) with 1, 8 and 2 as `thaum::casting::player_hooks`
  models.
- The space query's AI caller is the behaviour's method slot 26, called
  only by the story revive's world pass for every scene character: heal,
  contacts, two rounds of wander and home-side plans for a roaming AI,
  placement, the space query and the nearest walkable point.

## What Changes

- `exa_game::player::cursor_look`: the shared capture; `ChaseCamera::align`
  and its flag; the orbit's drag and close message read the capture.
- `exa_game::steering::Capture` in the behaviour's cursor routine.
- `exa_ai::presence::reenter` plans each round from where the last ended;
  `exa_game::revive_pass::reset_brains` runs it from the flow's new
  `ReviveBrains` step with the placement and the space query.

## Impact

Pointer moves cannot show in the offscreen grid; they are unit-tested.
Lockstep traces of exanima01 and the arena duel are unchanged.
