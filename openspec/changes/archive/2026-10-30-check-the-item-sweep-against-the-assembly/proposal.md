# Proposal

## Why

Phase 4 (Combat and items) of docs/PLAN.md. A sweep lane closed small gaps
across twelve item, material and crafting components. Its review against
the assembly found several of its rules wrong, duplicated or invented, so
this change records what was kept and what was corrected.

Evidence (REA, Ghidra listing of `Exanima.exe`, and the objdump listing):
the linear and lifted palette ramps (the step times 0.13 is a double
product stored as a float; the lifted sums and square and the saturation
lift are in double; the value cap is an ordered compare); the six paint
pass draws (the layer loop starts its counter at −1, binds layer i at unit
i + 2, i + 3 in the normal draw, and runs through the last layer; the
pattern picture binds at the colour layer count plus two; the normal draw
picks the alternate program on the item flag mask 8); the three steel
constructors' texture lists; the body-loot accept test (the AL crystal for
race flag 0x2000, crystal A for 0x1000, none while a core is worn) and
the equipment drop hook (the activation only for the AL crystal, the item
added either way); the item card's fade step (its one-shot call is the
GUI's forced pointer update); the arsenal constructor's eight-entry
control template (44 × 44 design pixels, picture indices, no captions);
the item manager's constructor (the icon-side global is unit × 40) and
destructor (the free-instance argument is compiler prologue).

## What Changes

- `exa_formats::palette`: the ramps' double-precision steps restored,
  with the ordered value cap kept.
- `exa_formats::procedural` and `exa_assets::paint`: the layer units start
  at 2 (3 for normals) and every layer is bound; the extra sampler is read
  from the program table.
- `exa_game::panel_live`: a body's equipment panel takes whatever core its
  race's test accepts, not only the AL crystal.
- `exa_ui::popups`: the one-shot step is named the forced pointer test;
  the duplicate close-request rule is removed.
- `exa_game::arsenal_view`: the controls sized from the template, the
  captions labelled as stand-ins, the not-wired state stated.
- Reverted: the item manager's construction and teardown model and the
  duplicate apparel copy walk.

## Out of scope

- Opening the arena arsenal window from the hub and filling its columns
  from an arena store; drawing its controls' pictures.
- The golem wake after a core activation (`CoreActivated` has no consumer).
- A GPU paint pipeline for the procedural material programs.
- Calling the inventory bind and close rules from the live window.
