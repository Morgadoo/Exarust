# Spec Delta

## ADDED Requirements

### Requirement: Item card coin icons
The item card's coin control SHALL draw each present denomination of its value (gold, silver, copper) as the coin sheet's icon at the coin manager's side (four GUI units) followed, four units later, by its count in the label group's sixth ink; the next denomination SHALL start two units after the count's text. The counts SHALL sit on the control's text baseline and the icons SHALL stand on it, in white at the card's GUI alpha.

#### Scenario: A cudgel's price
- **WHEN** `exarust grid "play exanima01 --story-session --give 282,23,243 --press I@100 --item-card 282@200 …"` runs
- **THEN** the card's Price row shows a copper coin icon before its count

### Requirement: Rating dots ink
The item card's and the inventory's rating dots SHALL draw in the label group's seventh shade.

#### Scenario: Encumbrance dots
- **WHEN** the inventory shows the player's encumbrance dots
- **THEN** they take the label group's seventh shade
