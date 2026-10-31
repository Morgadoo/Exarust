# Proposal

## Why

The inventory's character view (Phase 4, Combat and items; see docs/PLAN.md)
can be dropped on, but nothing can be taken off it, and a right press on the
figure opens no card. The rules are ported in `exa_ui::character_view`:
the drag-start plan, the hover pick's decision and the release plan. But
nothing calls them, and the pick had no per-node hit.

Evidence (REA, Exanima.exe):
- the character view's hover pick, drag-over, card and release routines;
- the scene meshes' ray callback;
- the skin builder's two append routines and its renumber pass, which fill
  and permute the merged mesh's per-face item table;
- the dressed-mesh builder, which passes each worn item's id as the append
  tag and 0 for the body's parts;
- the underwear re-add after a take-off.

## What Changes

- `exa_ui::character_view`: the hover pick takes the nearest hit. The ray
  first meets the merged body mesh, whose per-face table gives the worn
  item, starting from the largest float. Then it meets each node, but only
  when the node is nearer, within the ray's length. A node hit sets or
  clears the slot. The fallback flags come from the record's worn list
  (it was labelled the carried list before). Also new:
  - `ray_face`, the meshes' one-sided face test with its determinant
    threshold;
  - `nearest_face`;
  - `BodyHit`.
- `exa_assets::merge`: `MergePart::tag` and `Merged::triangle_tags`, the
  per-face table, carried through the material regroup.
- `exa_assets::model`: `DressedPart::tag` and `run_tags`.
- `exa_character`: `FaceGarments` on each merged run. Each face records its
  garment number, and the garment models are kept. Exarust's garments name a
  model, not an item.
- `exa_game::figure_pick` (new):
  - the pick on the live figure: the CPU-skinned merged runs, then the other
    drawn meshes, where a held item's meshes give the hand's item and slot;
  - the left-press drag start (`drag_start_plan`), as a `PanelDrag` from
    `Panel::Figure`;
  - the drop on a pack: the panel's accept test and drop, with the player's
    pack linked to the view;
  - the view's release (`release_on_figure`): the scheduled hand take-out
    with its promotion, or the garment taken off the worn list, with the
    underwear rule, the outfit garment removed and the pack charged its bulk;
  - the right-press card;
  - the debug flags `--figure-drag` and `--figure-card`.
- `exa_game::panel_live`: `Panel::Figure`, `ScriptedRelease`, and the
  release path for a drag off the figure.
- `exa_game::item_card`: the `FigureCard` request.

## Out of scope

- Re-dressing a character that is already dressed. Exarust dresses a body
  once, so the open figure keeps its old look until the inventory is opened
  again.
- The off-hand and torch models on the figure (lane cl/checkfig). Their
  roots can be marked `FigureHeld` for the pick.
- The dragged icon drawn under the pointer, which panel drags lack as well.
- The core crystal's shown-item path.
- The frozen, locked and mind-cost views: no running screen opens them.
