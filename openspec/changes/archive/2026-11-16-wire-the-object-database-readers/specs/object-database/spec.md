## MODIFIED Requirements

### Requirement: Oldest body conversion
The `0xDBAC0D06` body converter SHALL keep the blocks below `0x1A0`, copy the power word from the old bytes `0x1AC`–`0x1B3`, map the old category to the armour type (1–8 to 1, 2, 3, 1, 2, 4, 5, 6; others keep the byte), apply the category's finish changes when the finish is 1–7 without bit 7, write the count byte through the count setter with 1, draw one paint seed per body as a 63-bit value modulo `0xFFFFFFFF`, and turn the four old 16-byte parts into the first four appearance layers.

#### Scenario: An unknown category
- **WHEN** an old body's category byte is 9
- **THEN** the converted armour type is 9

#### Scenario: The seed from the generator
- **WHEN** an old body is converted with a twister seeded with a value
- **THEN** its paint seed equals that twister's first 64-bit draw below `0xFFFFFFFF`, and its count bits read 1

### Requirement: Operative use script
An operative section's body SHALL keep, after its fixed words, a use script: its length at `0x38`, read signed, and its text from `0x3C`, read as the database's counted list of the body; a length under 1 or past the body's end SHALL give none. In `objdb.rdb` of the installed build exactly one record holds one: the PedestalHand (`0x587`, class 2 kind 3, sound `DeviceA01`), `WitnessRole(Operative, $E1D1);`.

#### Scenario: The pedestal hand's body
- **WHEN** an operative body of class 2, kind 3 with sound `DeviceA01` and a 27-byte text at `0x3C` is read
- **THEN** the item's use script is that text; with the length 0 or −1 it has none
