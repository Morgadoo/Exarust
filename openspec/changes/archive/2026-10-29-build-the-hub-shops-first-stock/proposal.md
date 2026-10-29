# Proposal

## Why

docs/PLAN.md, Phase 7 (game modes and content), arena paragraphs: the hub
was wired, but its shop and merchant opened on an empty shelf (the first
stock build needed a live object-database world), and the ArenaMach side
was a guess (the node's origin x).

## What Changes

- `exa_game::arena_hub_live`: the ArenaMach side is the x of the clicked
  node's world bounding centre (`PartBounds` through the node's transform);
  only a side above zero opens the board.
- `exa_game::shop_stock::LiveShopStock`: the resource pack's `objdb.rdb`
  read once on the first shop click into shop entries (id, the table's
  flags word, the shop fields); apparel and weapon copies are new instance
  ids remembering their source; the shelf's names come from the item
  catalogue.
- The hub click builds an empty shelf's first stock: the shop with the
  default kinds and any rank, the merchant with its stock type and
  specialisation (`ArenaCareer::merchant_kinds`, `merchant_rank`, 0 and −1
  in a new career), which the save's merchant chunk now loads and writes.
- Review fixes on the lane's commit: the kinds word is the table's flags
  word, not a re-walk of the payload's section tags (`section_flags`
  removed); the merchant's draw takes its own rank rather than any rank;
  duplicate wrappers removed.

## Evidence

The retail hub click hook: the side is the clicked node's float at the
node's world bounding-centre slot (the same centre the collision broad
phase and the GI splat read), compared with `COMISS` against 0 behind
`JP/JBE`; the shop branch calls the stock update with (stock, 0, −1) when
the shelf count is 0, the merchant branch with the two merchant globals
the new-career reset sets to 0 and −1 and the merchant chunk's writer and
reader store as its first two words. The stock update reads each table
entry's second word as the kinds it tests. On the shipped `objdb.rdb` the
table word equals the OR of the payload's section tags for 1422 of 1423
records.

## Out of scope

The copies' item bodies (condition and weapon variant rolled onto a body),
moving shelf items into the trade baskets, the trade window's icons and
preview, and the merchant's category and rank dialog in the running game.
