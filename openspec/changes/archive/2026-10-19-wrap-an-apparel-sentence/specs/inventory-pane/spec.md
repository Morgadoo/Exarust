# Spec Delta

## ADDED Requirements

### Requirement: Apparel sentence
Id -1 SHALL keep the roll and use the fixed-name edit. A ] SHALL drop 4 letters. p and not c SHALL use They are. c SHALL add It is and skip quality. Else a bracket SHALL use It is. Same side of 50 SHALL join ", but", else " and". Leading "A " SHALL be stripped. A non-final dot tail SHALL follow the phrase. Only 14 made rows SHALL be read, then flawless, then perfect. Material 1 to 8 SHALL uppercase the side and replace a 0x10 pair with the name and a space. Any other 0x10 SHALL be deleted.

#### Scenario: A coat, a bracket, and a material
- **WHEN** "ab" is wrapped, "axe" is fixed, "coat" uses "well" and "sharp", "edge" is the made phrase, "Coat[p!]" uses quality 0, then 131, then 131 and 131, "Coat[x!]" is wrapped, "ax" plus a mark plus "e[pc]" is crafted, "a coat" and "coat." are wrapped, "A coat. lined" is wrapped, "ab" plus a mark plus "xy" has material 9 and then material 1 named "wool", a mark before "cde" uses "oak", and 14 missed rows hide a 15th "yes"
- **THEN** "ab" stays at the same roll, "axe" is "An axe" at the same roll, "coat" is "A well coat sharp." at roll 0x08088406, material 1 on "coat" keeps that text and the side becomes "Wool line", "edge" is "An edge coat sharp.", quality 0 is "CoatThey are well, but sharp.", quality 131 is "CoatThey are well and sharp.", both high marks are "CoatThey are well, but sharp.", the plain bracket is "CoatIt is well, but sharp.", the crafted mark stays in "ax" plus a mark plus "eIt is sharp." at roll 1 and the side stays, "a coat" is "A well a coat sharp.", "coat." is "A well coat sharp.", the dotted coat is "A well coat sharp. lined", material 9 becomes "A well aby sharp." and keeps the side, "wool" becomes "A well abwool y sharp." with side "Wool line", "oak" becomes "A well oak de sharp.", and the hidden row leaves "A  coat ok." at roll 1
