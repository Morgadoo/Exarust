## ADDED Requirements

### Requirement: Worn special kinds
The character build SHALL dispatch each worn item's special kind in wearing order: kind 2 grants the second wind, kinds 3, 4 and 6 register their item effect with the item's crystal flag and charge bytes, kind 5 wards the mind, and kind 7 sets the focus regeneration bonus to the float with bits 0x38D28EB4; kinds 0 and 1 do nothing here. This SHALL hold for every item of the shipped object database, which carries kinds 2 to 7.

#### Scenario: Mixed outfit
- **WHEN** a body wears, in order, kinds 1, 3, 4, 6, 3, 7 and 7
- **THEN** the registered effects are the two healing items, the suit and the gauntlets in that order, there is no second wind or ward, and the bonus has bits 0x38D28EB4

#### Scenario: Ward and wind
- **WHEN** a body wears a kind 5 and a kind 2 item
- **THEN** it is warded and has the second wind, with no focus bonus

### Requirement: Mind ward in play
A spawned body wearing a mind ward SHALL be perceived as hidden by other characters, and its caster SHALL refuse every power it casts; a focus charm SHALL set its caster's focus regeneration bonus. Without them the refusal bit and the bonus SHALL be cleared.

#### Scenario: Caster
- **WHEN** a caster is given a ward and a charm
- **THEN** its refusing bit is set and its bonus has bits 0x38D28EB4, and both clear when it is given neither
