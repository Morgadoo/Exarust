## Context

The original's human body (`humanbase.rfc`) has no underwear, and its skin
textures are painted nude. Underwear is an item: object-database record 8,
"Braies_A", which has a name section and a body but no ground-model
section. The rule that puts it on belongs to the character record: if the
record's race byte is 0 (human) and no worn item has the groin bit in the
pelvis entry of its innermost layer, item 8 is appended. It runs when a
record is created, when the character screen opens, after every equipment
removal, and in the arena's outfit generators. It does not run at spawn: a
placed record is dressed exactly as stored. A fresh record wears `[8]`,
including the play-start fallback with no player file, and story background
0 starts in braies only plus a start letter. Braies give way to anything
that covers the groin (trousers, hose, braies) and vanish when displaced;
they are never carried.

The research notes, with addresses, are git-ignored
(`context/re/underwear.md`, `context/re/underwear-re.md`). The format page
this change extends is [`docs/formats/OTHER.md`](../../../docs/formats/OTHER.md)
(Object database items and apparel painting, new subsection Underwear).

## Goals / Non-Goals

**Goals:** read the worn model and the occupancy table; dress, paint and
look up apparel by its worn model; keep the player and the character view's
human body in underwear by the original's rule; leave placed characters as
stored.

**Non-Goals:** inventory and equip, creator kits and backgrounds, saves, the
arena generators, fit variants and cull bits, the bare pelvis's black patch.

## Decisions

- **Worn model from the body.** `Item.worn_model` is the first 16 bytes of
  the `0x1000` body, the `Characters.rpk` entry the item is worn as.
  `Item.model` keeps the `0x00100000` section and is re-documented as the
  ground or icon model, which may be empty. The two agree for 1617 apparel
  records; they differ for item 8 (no ground model), for "Legg Cloth Lay A"
  and "Trouser Qui A" and their 39 level-local copies (worn as `Trouser_A`,
  shown on the ground as `Trouser_At`).
- **Occupancy.** `Item.occupancy` is the table at body `+0x10`, one `u64`
  per bone 1..20, byte L holding the sub-parts occupied on layer L. A body
  too short for it gives zeros, so the parser stays total.
  `Item::covers_groin()` tests bit `0x02` of the pelvis's layer-0 byte.
  Among real items, "covers the groin" is the same set as "overlaps the
  braies"; the only exceptions are two editor folders without a model.
- **The rule, written once.** `needs_underwear(race, covers)` is true for a
  human none of whose worn items covers the groin. Future equip and record
  code calls the same function.
- **Who carries the rule [choice].** A marker component, `WearsUnderwear`,
  on humans Exarust makes at run time. `PlayerBody` requires it: Exanima's
  player characters are human, and the original applies the rule to the
  player's record at creation and on every equipment change. The character
  view adds it to a human body, treating the view like the original's
  character screen; the bare body stays viewable with `exarust model
  Resource humanbase.rfc`. Placed characters never get it.
- **Before dressing, contents only.** A system runs before `equip_outfits`
  on marked, undressed outfits that were just marked or changed, and puts
  the underwear at index 0, as a fresh record holds `[8]` before the kit
  goes on over it. It changes what an outfit holds, never how it is
  dressed, so dressing fits and paints the braies like any other garment,
  and it marks the outfit changed only when it adds them. The braies count
  as their own cover, so the rule is idempotent. A model no item is worn as
  counts as not covering.
- **Groin cover by worn model.** For garments given only by model, cover is
  taken from the first item worn as that model. No model's items disagree on
  the groin bit (census in this change).
- **Item information by worn model.** Apparel appearance, hidden slots and
  armour are keyed by worn model; weapon sections stay keyed by the ground
  model. Among `Characters.rpk` models only `braies_a` changes: item 8 comes
  before "Braies_P" (425, mauve silk, used by nothing) in table order.
- **Placed characters** take `worn_model`, so item 8 and the `Trouser_At`
  items resolve; the held item keeps the ground model. No rule is applied.
- **Layering and armour.** Braies are layer 0 (pelvis `0b11`, both thighs
  bit 0) and hide no body part. Higher layers (mail legs 1, shirts 2, tunics
  and coats 4, plate 6–7) are separate meshes drawn over them by depth. They
  enter the armour layers in outfit order like any garment; their
  protection bytes are zero, so folding them changes no protection value.

## Risks / Trade-offs

- Outer garments do not cull braies triangles in Exarust, as no apparel
  applies cull bits yet; where an outer garment sits tighter than the
  braies, the braies can show through or above it.
- The braies render darker than the original's. Their shape matches the
  reference screenshots, but under the torch they read slate grey where the
  original's are pale grey, brighter than the skin. A probe that uploaded
  the painted colour texture as linear instead of sRGB matched the
  reference. That points at the painter's colour space, which every painted
  garment shares, so it is left to its own change (open question below).
- The torch: the original gives background 0 a letter, not a torch, while
  Exarust's play mode keeps giving the player its torch (existing "Carried
  torch" requirement).

## Open Questions

| question | how it will be answered |
|---|---|
| Does the armour-layer builder fold the worn list in stored order, braies first for a fresh record? | REA on the armour-layer builder; only the inner hit-sound class can differ, as the braies' protection bytes are zero |
| Do outer garments cull braies triangles? | `exa-tool chunks Characters braies_a` for the cull bits, REA on the apparel merge, a capture of the running game under Proton |
| Is the rule's race test the race byte Exarust reads? | REA on the record load; the corpus is consistent with it |
| Wear conflicts between arbitrary garments given on `--outfit` | Phase 4 equip, with removing braies when trousers go on and re-adding them on unequip |
| Is the painted colour texture linear or sRGB in the original? | REA on the apparel paint pass (its render target's format and how the item shader samples it), then a capture of the running game against `exarust character --outfit doublet_a` |
| Why does the bare pelvis draw black in Exarust? | its own investigation of the pelvis part's skipped `_null` seam triangles; the original never shows it |
| What does the `knight` background wear? | it has no actor template in 0.9.5; a capture of the creator if it unlocks |
