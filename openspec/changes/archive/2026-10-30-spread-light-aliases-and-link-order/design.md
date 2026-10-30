# Design

## Behaviour

- **Published light.** The sun setter stores the record, writes its direction times w and √w, then zeroes the reach and re-publishes the held one. The reach setter publishes when `|old − new|` (single) is not below 0.01 (double), scaling the sun direction by the reach, halved while the mode flag is clear.
- **Link sort.** Each entry after the head is unlinked and walked back while `before.weight < entry.weight` (comiss; parity exits), then inserted behind the entry it stopped at, or at the head.
- **Alias gate.** Only with the option word's bit 0x40000: the bounding sphere against the sector box's sphere, then the squared point-to-box distance against r²; a NaN falls through to reaching.
- **Find or make.** A sector alias of the same light with the light's stamp is current (nothing returned); an older one is refreshed (stamp copied, marked moved); none makes a new moved alias and marks the sector's light-added bit.
- **Spread.** The owner sector's busy byte is set, every linked sector not busy gets find-or-make, the walk recurses into those that returned an alias, and the byte is cleared.
- **Cut and edge.** `ends < 1e-4` closes the cap, anything else (NaN) turns and chains; an edge length `(x² + y²) + z² < 1` is skipped, anything else (NaN) makes a plane.

## Model

The scene model keeps aliases in `Sector::aliases` with a `stamp`, the link list on the light node, `Sector::linked` and `Sector::spreading`. The gate uses the light's world sphere and the sector node's box (both in sector space); a sector without a node is taken as reached (labelled guess).
