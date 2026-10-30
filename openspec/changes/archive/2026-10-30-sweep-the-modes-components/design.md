# Design

## Loadout constructor

The constructor builds the form's 28 control records from the shipped table
and then writes record fields for mode words 1 and 3. Laid out by the
record stride, the state-word writes land on records 4..9 and 11..20
(0x110, the hidden state the control draw skips) and 22..27 (0, shown) for
mode 1, which also sets Previous's width and Next's x and width. Mode 3
writes the disabled bit (0x10) on records 1, 6 and 11..14 and the hidden
state on 2, 3, 4 and 7. The lane had read these writes as a 24-slot
equipment table with 0x110 as "shown"; the records' text (HIRE, Hire Fee,
Upkeep, Profession at 22..27, hidden in the table) shows the mode-1 block is
a hire view, so Exarust's name `Creation` for word 1 is kept but documented.

## Arsenal return

The equipment drop handler first scans the form's own arsenal list for the
item lent under the form's mask (only for an item of the form's category);
only when that fails does it call the arsenal unit's lend query, and only an
unlent item of another category is written back with flags 0 (a new entry of
40, 4 on the shelf's first box). The lend query's outer loop over the five
boxes continues after a hit, so each box gives back its first lend.

## Live experience

The experience unit's tick is a hook the flow calls once a second; it
converts the account, then the player body's own record, then each
companion. Exarust's flow already emits `Step::Discovery` on that cadence
from the game clock, so the step application sends `DiscoveryTick` and the
live system converts on it, writing the learner back to the player's
`CharacterData`.

## Tip banner and card

The tips show routine draws from an 18-entry table and reads hints from the
same pointer table from its 18th entry on, so hint n (from 1) is entry
17 + n; the table order matches the string order. The tips window is hidden
by the end menu's hide routine and the prelude close. The divination card
draw calls the engine's cached cull-face setter with 0 at its start and 1
at its end, so both sides of a turning card are drawn.
