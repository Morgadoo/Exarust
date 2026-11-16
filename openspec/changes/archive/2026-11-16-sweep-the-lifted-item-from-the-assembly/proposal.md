## Why

Roadmap B4, SG_PlayerControls (lane playerctl2). The lift's placement sweep
was ported from the pseudo-code: the sweep met only the level, started no
nearer than the eye, kept the pick when the contact lay beyond it, and put
the item's grab point (the clicked point) on the ray. The per-sphere
pull-back the pseudo-code shows was read as a level support plane and had
no caller. The controls-off frame placed the lift by the controls' own
rules.

## Evidence

REA (Exanima.exe), read in the assembly:
- The free placement (the request's, and the GUI drop's) starts the item's
  box centre (its node's local box centre turned by the cursor's frame)
  600 cm back along the ray from the placed point (f32 -600) and moves it
  by the sweep's answer along the ray, whatever the pick's distance: the
  answer starts at 1000 (f32) and is kept when nothing is met.
- The sweep, for the held body and every linked body (their frames
  carried with the held one): each sphere's pull is
  `(b·eye − near − b·T) + r − c·(Rᵀb)` (sums x + y, then z) over `b·dir`
  when under zero (COMISS, NaN not), with `b` the camera's back axis (the
  third row of its matrix), `eye` its position and `near` its near plane
  (the value the screen unprojection uses): a sphere wholly in front of
  the near plane starts where its far side reaches it. Each sphere then
  sweeps from its pull over (best − pull), against every sector's mesh
  nodes with the collision flag (not the held node, not a rigid body of
  the held body's root), and a contact `(length + pull) / scale` replaces
  the best only when strictly nearer (COMISS, NaN not).
- The lift placement's tests: a hit strictly over the stand height is kept
  (COMISS, NaN false), a ray whose height part is over −1.19209e−7 refuses
  (NaN proceeds), and both come after the grid's occluder query, which
  refuses first; the markers are tried only after those.
- With the controls off the controls' frame is swapped for the hand's
  (the controls' on/off routine moves the two frame hooks): it turns the
  item, places it at the −100000 cm limit with the same sweep, no markers
  and no reach test; a fitting placement thaws a frozen item. Blocked or
  second-blocked, the controls' frame returns before its request.

## What Changes

- `item_cursor`: `near_plane_pulls` (the reading corrected), the pull in
  each sphere, the sweep over what is left to the best, `sweep_distance`
  (the whole reach on no contact), `sweep_start`, `centred_frame`.
- `hand::hold_hand`: the free placement puts the box centre on the swept
  point and copies the frame into the held node whole; the sweep meets the
  level, the props' stand-ins (the held body's left out) and the loose
  props' meshes; the controls-off frame; the blocked frame.
- `exa_physics`: `Physics::solid_sweep_except`, `body::pairs::ball_cast`,
  `Physics::of_level_meshes`, `PropBody::new`.
