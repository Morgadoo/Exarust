# Spec Delta

## ADDED Requirements

### Requirement: Armour class sets
A part's limb impact and a character's clothing foley SHALL take their sets by the layer's hit-sound class in the item byte's order: the part impact plays the flesh default `bdycol` (`SkelHit` for a skeleton) for class 0 and `hitclt`, `hitltr`, `hitchn`, `hitplt`, `hithbr`, `hitglm` for classes 1 to 6; the foley loop plays `foleyclt`, `foleyltr`, `foleychn`, `foleyplt`, `foleyhbr` for classes 1 to 5, with factors 0, 0, 0.3, 0.6, 1, 1 for classes 0 to 5, so flesh and cloth never loop.

#### Scenario: Worn items
- **WHEN** a part's outer layer is a breastplate (class 4), a mail shirt (3), leather boots (2) or cloth (1)
- **THEN** its limb impact plays `hitplt`, `hitchn`, `hitltr` or `hitclt`

#### Scenario: Foley
- **WHEN** a body is covered all in cloth, then all in plate
- **THEN** it has no foley loop, then it loops `foleyplt`
