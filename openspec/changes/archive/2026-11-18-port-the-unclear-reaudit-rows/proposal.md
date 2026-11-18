## Why

Roadmap task B4, lane `fixes4`: the re-audit of the ledger's UNCLEAR rows
(`context/artifacts/2026-11-18-unclear/port-items.tsv`) turned 63 rows into
concrete port items. This change ports the player-visible ones first: the
dressed merge's garment paint, the hit resolver's strike speed and fist
measure, the get-up's twist reset, the burn's groan, the hair cloth's two
culled passes, the parchment's block sample, the skill screen's armour
refresh, the teleporter's arrival, a refused AI cast, the manager's health,
the shell's break sound, the light flood's box clear and the terrain's
texture-detail pools.

## Evidence

REA (Exanima.exe 0.9.5), read in the assembly: the dressed merge's group
match and its culling append, the hit resolver's opening (the striker slot
loop, the stab's axis dot, the fist's hand test and its unarmed zone), the
get-up start, the burn's crackle, the hair cloth's draw procedures, the
parchment's texel sample, the skill screen's close, the content stream's
arrival words, the power system's AI cast answer, the arena's health set
and its callers, the force shell's close step and the light flood's clear.

## What changes

- `exa_assets::merge::MergePart::group` and `SubObject::group`: a painted
  garment's `&` parts form one run per garment; `model::merge_dressed`
  maps runs to handles by that key.
- `exa_combat::blade_strike` takes the striker slot's swing tracker
  (`tracked_strike_speed`); the fists' striker frame turns by hand.
- `exa_game::balance::start_getting_up` resets the torso twist.
- `exa_combat::burn::burn_upkeep` asks the voice for its groan.
- `exa_world::cloth_draw::draw_hair_passes`; `ExaMaterial::second_pass`.
- `exa_game::parchment` samples by texture block.
- `exa_game::skill_screen::close` inserts `ArmourRefresh`.
- `exa_game::level_change::journey_placement`, used by the viewer.
- `exa_game::thaum::cast::cast_for_duellists` clears a refused cast.
- `exa_game::arena_hub_live::apply_manager_health`.
- `ForceShell::break_set`, played by `tick_shell_auras`.
- `exa_world::light_flood::CellLights::clear_box`.
- The terrain composite pools' detail size from the texture setting.
