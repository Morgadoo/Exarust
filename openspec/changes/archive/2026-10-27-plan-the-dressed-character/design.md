# Design

The dressed mesh stays a plan: `dress` returns `MergeStep`s that a caller
with model meshes turns into `exa_assets::merge::MergePart`s. Two walks are
kept apart because the original keeps them apart: the weight pass walks the
slot tree (`runtime::merge_order`), the append loops walk slot numbers
(`dress::garment_steps`, `body_steps`, `body_only_steps`).

The impact material follows the coverage pass's own words
(`GarmentPass::uncovered`), compared unsigned with a strict less-than so
the first garment in layer order wins ties.

The attach plan reads the equip result rather than re-deciding kinds: a
shield from either hand lands in the off-hand holder, so the off hand hangs
the shield first, else hand 1's torch, else hand 1's weapon. The flipped
grip is a single character flag (`holds_flipped`).

The apply plan's hair word is modelled as `Option<&[usize]>`; its exact
meaning (non-zero removes, positive attaches) is a labelled guess.

Nothing new is wired into the running game: the dressed mesh, spawns and
apply step have no caller until the renderer draws one mesh per character.
