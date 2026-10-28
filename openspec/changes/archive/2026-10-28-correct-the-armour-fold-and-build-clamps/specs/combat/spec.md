## ADDED Requirements

### Requirement: Armour fold precision
The armour pass SHALL compute a layer's effective coverage as (1 − c)·((c − ½)·1.75) + c in float above one half and c otherwise (a NaN keeps c), the natural layer's coverage and padding as double products narrowed once and its resistances as float products, and the folded resistance as (1 − s0′)·(s1·γ1) + s0′ with s0′ = γ0·s0.

#### Scenario: Fold grouping
- **WHEN** a part is covered by items of 80 % slash 30, 51 % slash 90 and then any third item
- **THEN** the inner layer's slash resistance is the float `0x3eece03c`

#### Scenario: Effective coverage
- **WHEN** a layer covers 0.8 of a part
- **THEN** its effective coverage is the float `0x3f67ae15`

### Requirement: Second wind from worn items
A character SHALL be granted the second wind when one of its worn items, found as the encumbrance finds it, has special kind 2; placed characters and scene bodies SHALL both resolve it.

#### Scenario: Kind-2 item
- **WHEN** a record wears an item whose special kind is 2
- **THEN** its fighter's second wind is granted, and a record whose worn ids find no such item is not
