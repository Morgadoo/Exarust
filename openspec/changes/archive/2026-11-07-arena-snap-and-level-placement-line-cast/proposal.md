## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, motilepart2 lane,
second pass (round 97). Round 95 ported the line cast and its start-of-play
caller; the other two callers were left.

## Evidence

REA on `Exanima.exe` 0.9.5:

- The arena snap (cast from 1000 cm up, 3000 cm down, the hit plus 5 cm,
  a miss keeping the point) has six callers: the generic opponent spawner
  and the tournament profile spawner, the Beast's spawner, the company
  entrant placement (every match setup and the reserve's next round reach
  it), and the elimination and reserve setups, which snap our spot. The
  generic spawner snaps the placement matrix's position row before it adds
  the body, so the body stands at the snapped point.
- The level load by name (the save-load and level-change path) places the
  player at the level's entry matrix and calls the save unit's placement:
  raise 100 cm, cast 300 cm down, store the hit as the ground, then lift the
  frame by `108.18 − 11.1·extent` and 5 cm (13 cm in a raised state), place
  the character and re-place its held items; each follower the same at its
  offset.
- Traced, not changed:
  - **Status-effect list:** reachable. The broadcast reaches the body's
    effect objects with event 2 at death, 0 on the focus tick and 8 at a
    level change. Its entries are persistent-power objects whose event
    routines belong to the thaumaturgy units; Exarust's
    `condition::update_statuses` has no live list to run over.
  - **Pin and contact re-scaling:** reachable. The physique copy reaches it
    for parts copied from the template body, and the 0x600/0x700 races run
    it on one part at half the character size. Before it can be ported the
    parts' template bit and the half-size part need tracing.
  - **Node matrix snap:** the weapon attach places the item's child nodes
    and their part particles from the parent's world matrix. Exarust builds
    held item bodies from the placed nodes' transforms, so this is
    equivalent by construction [inf].
  - **Grip mode at attach:** the attach's reset (−1, then 0) is followed at
    spawn by the actor spawn's forced clip start, whose motion-start rule
    sets the clip's mode. Exarust's first tick after the grip builds counts
    as that start, so spawns already match. The mid-game equip reset and
    the AI's writers (the stab fallback at a facing penalty of 6 or more:
    ease mode 2 and grip mode 0; the chase route's mode 1) are not ported.
    The first needs an equip-time hook, the second the facing count, and
    the third is the route-walking lane's.

## What Changes

- `exa_game::scene_bodies::SceneBodyPlan::snap`, set by the opponent,
  hire, Beast and company plans; the scene command applies
  `exa_world::start::arena_snap` against the frame's replica.
- `exa_game::match_layout::snaps_our_spot`; the viewer snaps the player's
  spot in elimination and reserve matches.
- `exa_game::save_level::landing_feet`; the viewer's placement after a load
  stands the feet 5 cm over the cast's hit. Exarust's character stands on
  its feet, so the frame's standing height (108.18 − 11.1·extent) is left
  out [inf], and the raised state's 13 cm is not modelled.
