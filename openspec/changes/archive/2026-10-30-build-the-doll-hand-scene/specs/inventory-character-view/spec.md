# Spec Delta

## ADDED Requirements

### Requirement: Insight's off-hand drop
A weapon dropped on the right half of the character view (the view's width halved less than the pointer's view x) by a character who knows Insight's technique 8 among the first points / 5000 techniques SHALL be taken into the off hand.

#### Scenario: One-hander on the right half
- **WHEN** a one-handed sword is dropped on the right half by a player with 5000 Insight points and technique 8, the main hand holding item 8
- **THEN** the sword goes into the off hand beside item 8

### Requirement: Figure drag start
A drag started on the figure SHALL, with the mind cost charged, store the item's cost (twice a weapon's desirability, else an apparel's protection, else 0) and refund it, SHALL cancel when the character cannot take the charge, and SHALL otherwise answer 0x105 with kind 0xFB01 (all ones for a locked view's item flagged 0x80000000) and a payload of the icon, the item's flags and half the icon size; an icon that does not resolve SHALL start no drag while the charge stands.

#### Scenario: A weapon of desirability 1.25
- **WHEN** a weapon of desirability 1.25 with flags 0x10 and icon 4 is dragged at icon size 160 with the mind cost on
- **THEN** the cost 2.5 is stored and refunded and the payload carries kind 0xFB01, icon 4, flags 0x10 and half 80
