# Spec Delta

## ADDED Requirements

### Requirement: Burning blade from a socketed crystal
Once a game second, each held item whose weapon section names ability type 3 SHALL run the flame row's held check on its first socket word. A Power Crystal C in the socket (kind 4) with a charge below 254 SHALL light the blade when it has no live flame row:
- a flame row from 30 to 115 cm along the item's +Y, fading in;
- the row's warm light at the line's middle;
- the `FireS02` loop;
- the `_FlameMat01` ribbon over `flameletsa3`;
- the item's lit flag;
- the "Flaming" moveset overlay (0x10020000).

A cold socket SHALL put a live row out, clearing the lit flag and the overlay. A hit struck with a lit blade SHALL play `FireSizzleA` at the contact, at volume 0.3 and pitch 0.8 + 0.2·u.

#### Scenario: Crystal C into the shipped sword
- **WHEN** "Sword GSF 01" (socket word 0x0080) takes a Power Crystal C through the item GUI's drop
- **THEN** its socket word is 0x0084 and the next check lights the blade; with the crystal taken out the check puts it out

#### Scenario: Other crystals
- **WHEN** the socket holds crystal A, B or AR, or the item's type is not 3
- **THEN** the check lights nothing
