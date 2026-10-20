# Design

## Behaviour

`roll_rank` keeps an unranked template at −1; a flag-2 template takes the slot; otherwise the rank is the slot biased by `1−r⁴`, truncated. The rank window is 0–5 for Challenger and `slot..=rank` otherwise. The secondary value scales like cash: unpack base-12, multiply by `slot+1`, pack again.

Generation accepts a template when its per-pass cap is not reached, the slot is at least its minimum and the 0–7 reject roll misses the mask. The draw uses the same `0x8088405` generator as the item panel and retries; it returns `None` after `len·8` attempts rather than looping forever.

Settlement maps an outcome to `pays`, a log tag and the Beast counter; the played counter always rises. The season award needs week 3 and all four weekly wins.

## Model

`exa_game::arena` adds `preset_rank`, `roll_rank`, `rank_window`, `scaled_secondary`, `eligible`, `pick_template`, `pick_prize`, `settle` and `season_award`.

## Open questions

The live board, opponent generation, shop, tournament resolution and UI are separate.

## Docs

`XA_Arena` is **18%**. The headline rises slightly.
