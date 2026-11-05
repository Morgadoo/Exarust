# Design

## Forms

Every form goes through `paint_top_form` (owner the task) or
`paint_held_form` (owner a form) with its constructor's words; the commit
(`commit_words`) does the rest. Words traced:

| form | style | attributes |
|---|---|---|
| table view | 0x88 | base \| 0xc10 |
| item view | 0x88 | base \| 0x490 |
| text box | 0x8c | base \| 0x410 |
| Manage Company, match board | 0xb800 | base |
| board picture pane / reward panel | 0x82 / 0x80 | base |
| Settings | 0xb810 | base \| 0x1000 |
| Settings page / Controls page | 0 / 0x88 | base / base \| 0x280 |
| manual | 0xb800 | base \| 0x1000 |
| manual list / page | item view less 8 / text box \| 0xc0000 | item view / text box \| 0x280 |
| delete box | 0xb800 | base \| 0x1000 (border 0) |
| arsenal | 0 | base |
| arsenal column frame | 0x10 | base |
| arsenal columns | item panel | item panel |
| embedded inventory | 0x3010 | (base & ~2) \| 1 |
| card | 0x3080 | 0x310f0 (plain, no text: 0x830020, style less 0x80) |

The creator's preview is assumed to be the inventory's 3-D view class (its
builder's words match): style 0x88 \| 0xf0080, attributes 0x70092.

## Scroll bar

The scroll-bar branch reads the form's client right, top and bottom and the
thumb place the line skin's scroll refresh stores. Arrows: one unit plus
round(0.15 u) past the client's right edge, 2 (3 with the corner flush)
units from the client's top and bottom, two units square; thumb four units
square from a unit above its place. Every term is a whole number.

## Card

The builders append template records: label (style 0x1300, its caption
ending a unit left of x 156) and value (dots or slider at x 152) pairs one
anchor row apart, the added rows at x 120 from an absolute row. The layout
runs at unit 4 in the 244 × 236 client, the show rescales the card. The
title ink is the tint word's bytes in memory order (red first), alpha by
the fade.

## Open questions

- The slider's knob icon (its atlas index is a global set at the item
  GUI's start; not traced).
- The match board's item slots' class draw.
