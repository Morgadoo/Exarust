# Spec Delta

## ADDED Requirements

### Requirement: Fist cursor nudge
While a fighter's weapon class word is exactly 0, each striker whose window is open SHALL be pushed along the cursor, 10·(1 − 2|clock − 0.5|) cm a tick until a clock of 0.9. The fists (0x80), the claws (0xC0), two weapons (0x90) and any weapon class SHALL not be pushed.

#### Scenario: The fists are not pushed
- **WHEN** `exarust --no-torch --still-opponent --at 0,0,-1.1 --press LMB@120-300 play arenao1` punches
- **THEN** the debug log reads class 0x80 with one striker open and none pushed

### Requirement: Knockback step length
A step of the knockback kind SHALL reach (1.5 − 0.3·encumbrance) times the step length scaled by the leg extension, the scaled length in single precision and the factor in double.

#### Scenario: Encumbered
- **WHEN** the step length is 80 cm with full extension, at encumbrance 0, then 1, then extension 0.5 at encumbrance 0.5
- **THEN** the knockback step reaches 120, 96, then 54 cm
