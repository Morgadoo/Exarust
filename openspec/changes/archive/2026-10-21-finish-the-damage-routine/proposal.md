# Proposal

## Why

The damage routine of the original's character dynamics resolves every
weapon contact. Exarust's `damage::resolve` ported the flesh path's
formulas from the research notes, which left their arithmetic order and
precision open, and it dropped what the rest of the hit needs: the impact
at the hit, the zone flags the striker's follow-through reads, the bite's
smoothing, the blow sag, the daze's stall factor, the hit sounds. A
re-read of the routine's listing (not the pseudo-code) found the haft's
crush bonus wrongly gated, the softening, the stab's impact and tip
factor, the layer resistances and the daze in a different order or
precision than the original's, the crush and pierce powers taken with
the true power where the original uses its fast logarithm and
exponential, and the striker's reaction after a wound (cleave or follow
through, recoil from a guard or a body that turns blows) never run. The
weapon record's holding pass also computes several elements in single
precision where the original multiplies in double. Phase 4 (combat and
items), the damage model (`add-damage-model`) and the striker's side of
the hit (C21 territory, `finish-the-damage-rules` in the combat notes).

## What Changes

- `damage::resolve` follows the routine's order and precision: the speed
  cap and softening, the impacts (swing in double, stab), the powers'
  factors (none in Exarust: 1), the hit point, the four zone paths (stab,
  hanging striker, unarmed, armed with each kind's tip zone and the haft's
  material bonus), unarmed 0.6·√(3I), the coverage roll (only for a blow
  on a part), the channels with the fast power, the shove, readiness, the
  daze and its stall factor (Security, a potion, a steadfast body), the
  bars. It reports the impact at the hit, the speed, the total, the zone,
  edge, body and haft flags, the bite smoothing, the blow sag and the hit
  sounds.
- A quadruped's stab resolves as a swing (`CharacterStats::quadruped`).
- The hit reaction: the victim's rules feed the blow; a guarded blow
  draws no roll; a blow on flesh draws the struck part's sound cooldown
  number; a body that turns blows (state flag 0x400) takes nothing; the
  struck attacker's stall uses the blow's stall factor; the blow sag rides
  the shove to the struck body; then the striker's swing after a wound
  (cleave or follow-through by its size, the blow's impact and zone) or a
  recoil (a guard, a turned blow).
- The holding pass's elements in the original's precision (half-sword
  balance, h, W20, W15, W27, the unarmed swing sounds) and its non-number
  branches.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `combat`: Blow damage, Struck swing, Guarded blow; new
  requirements for the striker after a wound, a quadruped stab, a turned
  blow, the blow sag,
  the hit sounds and the holding pass's precision.

## Impact

- `exa_combat::damage` (`Strike`, `Victim`, `Outcome`, `resolve`,
  `guard_outcome`, `hit_sounds`), `exa_combat::fast_math`,
  `exa_combat::swing` (`SwingState::after_wound`, `recoil_from`,
  `Striker`), `exa_combat` (`detect_hits`, `react_to_hits`, `Roll::below`),
  `exa_combat::condition` (`StateFlags::TURNS_BLOWS`).
- `exa_formats::weapon_record` (`WeaponRecord::hold`),
  `exa_formats::character_stats` (`quadruped`).
- `exa_physics::body` (`Shove::sag`, `Ragdoll::sag_from_blow`).
- `docs/formats/OTHER.md` (Damage, Weapon record), `docs/formats/RSQ.md`
  (Pose hold), `crates/exa_combat/README.md`.

## Evidence

REA (Ghidra) on the damage routine's listing, its fast logarithm and
exponential, the generator, the vital-flag helper, the daze routine and
the follow-through, the holding pass and the equip refresh; constants read
from the executable. A Python model written from the listing pins the
tests' numbers.

## Out of scope

The weapon-on-weapon clash and the shield block paths beyond what the
guard does now (stamina, the parry locks, the Synchrony, Persist and
Counter perks); the bite link (weapons are not bodies); the hit, flesh
and pain sounds' playback (the volumes are computed); the camera shake
and the heartbeat nudge of the player; the script hit callbacks; the
Force Shell and Force Strikes powers; the attack-state bit 0x40 that
refuses a wound (its writer was not traced); the recoil's stance blend
and arm stiffness.
