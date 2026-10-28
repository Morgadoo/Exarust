## ADDED Requirements

### Requirement: Description phrase tables
The game SHALL read the five description phrase tables (11 flawless, 10 perfect, 8 high-quality, 6 great and 15 made rows, each a text and an inclusive 0 to 100 mark range) and the eight armour-variant words from the installed executable, finding each table by its first phrase, and SHALL ship none of their text. A table whose first phrase is also a row of an earlier table SHALL be the occurrence outside it. A malformed image SHALL be an error.

#### Scenario: Reading the tables
- **WHEN** a synthetic image holds the tables, the perfect table's third row repeats the great table's first phrase, and the image is then cut short or given a 32-bit header
- **THEN** every table has its row count, the great table is the one outside the perfect table, the variant words read in order, and the damaged images are errors; the installed executable gives every table with ranges within 0 to 100

### Requirement: Card sentence choice
The item card SHALL use the weapon sentence when it shows a weapon page and the model name starts with `@`, else the apparel sentence when the item has an apparel body, else the weapon sentence over the shield section when the model name starts with `@`, else none.

#### Scenario: Choosing the rule
- **WHEN** a record has a weapon page and an apparel body on a plain model, the same on a factory model, an apparel body with a shield, and a shield alone on a factory and a plain model
- **THEN** the first has no sentence, the second the weapon rule, the third the apparel rule, the factory shield the weapon rule over the shield's bytes, and the plain shield none

### Requirement: Card sentence
The weapon rule SHALL read the section's seed word, quality and damage; the apparel rule SHALL read the body's variant, seed, quality and damage. A seed of -1 SHALL keep the fixed name. The apparel rule SHALL put the variant word of variants 1 to 8, first letter capitalised, before the title. The rewritten description SHALL then pass through the condition blocks and pronoun markers. Without the tables the description SHALL stay as it is.

#### Scenario: A gambeson and a mace
- **WHEN** a body of variant 2, quality 200 and damage 10 describes "A gambeson." titled "gambeson", and a weapon of quality 30 and damage 200 describes "mace" and "-mace", over tables whose rows split at 50
- **THEN** the apparel text is "A finely made gambeson in mint shape." titled "V2 gambeson" after two draws, the mace is "A roughly made mace in rough shape." and the dashed mace "A top grade mace in grand shape."; a fixed seed keeps "Coat of plates" and the roll

## MODIFIED Requirements

### Requirement: Apparel sentence
Id -1 SHALL keep the roll and use the fixed-name edit. A ] SHALL drop 4 letters. p and not c SHALL use They are. c SHALL add It is and skip quality. Else a bracket SHALL use It is. Same side of 50 SHALL join ", but", else " and". Leading "A " SHALL be stripped. A non-final dot tail SHALL follow the phrase. All 15 made rows SHALL be read, then flawless, then perfect. Material 1 to 8 SHALL uppercase the side and replace a 0x10 pair with the name and a space. Any other 0x10 SHALL be deleted.

#### Scenario: A coat, a bracket, and a material
- **WHEN** "ab" is wrapped, "axe" is fixed, "coat" uses "well" and "sharp", "edge" is the made phrase, "Coat[p!]" uses quality 0, then 131, then 131 and 131, "Coat[x!]" is wrapped, "ax" plus a mark plus "e[pc]" is crafted, "a coat" and "coat." are wrapped, "A coat. lined" is wrapped, "ab" plus a mark plus "xy" has material 9 and then material 1 named "wool", a mark before "cde" uses "oak", and 14 missed rows precede a 15th "yes"
- **THEN** "ab" stays at the same roll, "axe" is "An axe" at the same roll, "coat" is "A well coat sharp." at roll 0x08088406, material 1 on "coat" keeps that text and the side becomes "Wool line", "edge" is "An edge coat sharp.", quality 0 is "CoatThey are well, but sharp.", quality 131 is "CoatThey are well and sharp.", both high marks are "CoatThey are well, but sharp.", the plain bracket is "CoatIt is well, but sharp.", the crafted mark stays in "ax" plus a mark plus "eIt is sharp." at roll 1 and the side stays, "a coat" is "A well a coat sharp.", "coat." is "A well coat sharp.", the dotted coat is "A well coat sharp. lined", material 9 becomes "A well aby sharp." and keeps the side, "wool" becomes "A well abwool y sharp." with side "Wool line", "oak" becomes "A well oak de sharp.", and the 15th row gives "A yes coat ok." at roll 0x08088406
