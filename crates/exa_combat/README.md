# `exa_combat`

Combat resolution: contacts between weapons and bodies, impact energy, armour coverage and layers, damage types and injuries.

**Phase:** Phase 4 (see [`docs/PLAN.md`](../../docs/PLAN.md)). Status: skeleton.

It already resolves blade contacts by the original's damage model
(`damage`) and keeps each fighter's combat stance and readiness (`stance`:
the looping clip it returns to, the body tick's switch to it that skips
Cmbt - Idle, the player's and a duellist's rules, readiness halving the
wound after 3 s in Cmbt - Idle; rules in
[`docs/PLAYER.md`](../../docs/PLAYER.md), The combat stance).

## Scope

- Hit detection from physics contacts (edge, point, blunt surfaces of a weapon).
- Momentum/energy based damage; blunt, cut and pierce through armour layers.
- Stagger, knockdown, grabs, blocking with weapon or shield.
