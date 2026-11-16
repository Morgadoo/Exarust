## Why

Roadmap tasks B1 and B4 (lane `datastream`): the object database's five
row-condition readers (wealth, damage and its spread, dirt and its spread
of a random table row's condition word) had no live caller. Their one
caller in the original is the container instantiate the world load runs
for each picked row, whose copy and condition Exarust's container fill
left out: a picked child named the row's item itself.

## Evidence

REA (Exanima.exe 0.9.5), the container instantiate (pseudo-code read, the
offsets checked against the assembly): an empty slot or a zero row writes
`{0, 0x100}`; otherwise the row's item goes through the database's
dispatcher, and when the result's record is found: with an item body, the
item palette is rolled on the row's wealth (kind 1), the count byte's
signed fraction takes the wealth less the count, damage and dirt take the
row's shares of 255 with their spreads in the packed byte, and the apparel
walker runs; without one, a world or local copy with a weapon section
takes damage and dirt and their spreads in the weapon's bytes and the
weapon walker runs. The child then takes the result's id with flags 0.

## What changes

- `exa_game::container_fill`: `ContainerPicks` (the instantiate over the
  live database, a `PickStore`), `fill_containers_with` (a pick per given
  row, in the draws' order); `fill_level_containers` picks through the
  live database when it has a base tier.

## Out of scope

The whole-database walk and the shared generator (the fill still walks the
placed containers from `FILL_SEED`; the walkers' clock words are a fixed
sequence, as the archive pass's).
