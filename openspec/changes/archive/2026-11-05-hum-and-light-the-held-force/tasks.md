# Tasks

## 1. Kinesis hum

- [x] 1.1 The pick names `thmfrc_knsdr` on its shell, its start drawn from the effect stream
- [x] 1.2 The voice starts at volume 0, pitch 1, looping, `r·0.8` of its length in; follows the node and the shell's levels; freed with the shell

## 2. Box light

- [x] 2.1 `held_box_shape`: extent, colour, `bMat`, world box plus 80 cm
- [x] 2.2 Every held shell registers it once its box is fitted; it leaves with the shell's light

## 3. Strikes

- [x] 3.1 `StrikeHooked` at the damage routine's three calls
- [x] 3.2 The primary weapon's aura ripples at half the strength, with `thmfrc_shi`
- [x] 3.3 `strikes_line` and `strikes_aura_shape`: segment, gradient, colour, point light, world box plus 60 cm

## 4. Proof

- [x] 4.1 Unit tests: hum start, node place, box shape (unturned and turned), strikes line and aura shape, cuboid MAXSS
- [x] 4.2 GPU: a held prop's box light and hum (open, rest, close, freed); strikes swings with ripples and the line's point light
