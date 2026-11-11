## Why

About twelve rounds landed after round 169's feature sweep: Raise Dead, the Mind casts reaching the live brains, the
balance routine's last blocks and the armed parry, fist hits, the force effects' contacts with the level's meshes,
the arena trade through the arsenal and Manage Company's tables, the archive pass, the scene scissor and the cell
order. Round 182 ran the sweep again on main plus round 181, free-running and in lockstep, added scenes for the
features it did not yet cover, and fixed what it found.

## What the sweep found

- **An AI's Force Blast never left its hand in free mode.** The frame draws the blast, torrent, barrier, bubble,
  barricade and wisp effects by writing into them (light points, ribbon normals, shown shares, growth). The bridge
  copied each changed effect back over the simulation's newer state, so the effect went back to an older step. The
  duellist's charging blast never reached its release, and its next motion dropped the cast hook ("cast hook
  dropped: the cast clip was replaced by Cmbt - Idle"). This happened in 4 of 4 free runs, with no release sound and
  no level contacts. Lockstep was unaffected.
- **The armed parry logged nothing.** No sweep scene could tell whether a held weapon's parry link took hold.

## What Changes

- The six force effects the frame only draws are registered as drawn copies (`bridge_copy_drawn`). When the
  simulation runs free, a frame copy based on an older step is dropped instead of overwriting the effect. Lockstep has
  no such conflict, so its traces are unchanged.
- The armed parry logs, at debug level, the tick an item's parry link goes from disabled to held
  (`GripLinks::parry_held`).
- The sweep's scene list (outside git) gains 16 scenes: the brains' Fear, Calm, Loyalty and Command Dead, the raised
  body, Spirit Form's return, the archive pass in two levels, a real exit's portal use in exanima03, the trade's buy
  and sell, Manage Company's drag and report line, fist strikes, the armed parry, and the blast's and the torrent's
  level contacts.

## Impact

- `exa_game::thaum::live` (bridge registration), `exa_combat::parry`, `exa_physics::grip`.
- Lockstep traces of exanima01 and the arena duel are unchanged.
