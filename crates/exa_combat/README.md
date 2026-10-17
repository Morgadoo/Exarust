# `exa_combat`

Combat resolution: contacts between weapons and bodies, impact energy, armour coverage and layers, damage types and injuries.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already decides when a fighter may attack and which of its strikers
strike, by the original's attack executor shared by the player and the AI
(`attack`: 80 % of the most energy, the cooldown from the last attack tick
on the 12 ms game clock, 400 ms or round(300 × the primary's recoil) +
300 ms, the choice of attack, the motion's hand with the fists; rules in
[`docs/PLAYER.md`](../../docs/PLAYER.md), Combat mode), and resolves the
contacts of blades and fists by the original's damage model
(`damage`, with the weapon record of `exa_formats::weapon_record`
re-exported as `damage::WeaponRecord`) and keeps each fighter's combat stance and readiness (`stance`:
the looping clip it returns to, the body tick's switch to it that skips
Cmbt - Idle, the player's and a duellist's rules, readiness halving the
wound after 3 s in Cmbt - Idle; rules in
[`docs/PLAYER.md`](../../docs/PLAYER.md), The combat stance).

## Scope

- Hit detection from physics contacts (edge, point, blunt surfaces of a weapon).
- Momentum/energy based damage; blunt, cut and pierce through armour layers.
- Stagger, knockdown, grabs, blocking with weapon or shield.

## Wounds

`wounds`: a blow bleeds when the struck fighter's race bleeds
(`Fighter::bleeds`, from `races.rdb` flag bit 0 through
`Fighter::of_record`) and its wound's size R (`Outcome::wound`, the lasting
damage on the part before the red bar's 0.75) exceeds 0.05. The wound waits
on the victim (`WoundQueue`, seven at most, a new one dropped when full)
with the striking body (`Hit::striker`), the contact point as weights in
its tetrahedron, the world point and R. Each body tick counts the waiting
wounds down; seven ticks after the blow the first is sent as a `Bleed`
(one a tick, corpses too) with how the striker's struck point moved on
since and the radius 30·R + 10 cm; `exa_character::blood` paints it. See
[`docs/RENDERING.md`](../../docs/RENDERING.md) (Blood).

The weapon that deals a bleeding blow draws blood at the blow
(`wounds::BladeBlood` on the entity holding its model): half the wound's
R, up to 1, never washed off, even when the wound itself is dropped from a
full queue. Fists have none. `exa_character::blood` draws its stain
([`docs/RENDERING.md`](../../docs/RENDERING.md), Blood on weapons).

A bleeding blow also sprays at the blow (`wounds::Spray`, a message): one
jet on the struck part at the contact point, against the striking point's
swing, of round(0.3 × strength) droplets (ties to even) with strength 120·R,
or 200 once R exceeds 1.67 (`spray_strength`, `jet_droplets`): 36·R
droplets, 60 at most, even when the wound is dropped from a full queue.
`exa_character::spray` flies them
([`docs/RENDERING.md`](../../docs/RENDERING.md), Blood sprays).
