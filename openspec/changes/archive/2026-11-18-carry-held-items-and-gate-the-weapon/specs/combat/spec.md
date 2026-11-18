## ADDED Requirements

### Requirement: Weapon-in-hand gate
Once an attack, after its clip's lead-in, an armed fighter's swing SHALL wait for its primary weapon's held body to be within 50·scale cm of where its hand holds it with its long axis within 60° of that place's; until then each tick takes 0.1 off the clip rate (when above 0.1) and the swing tick goes no further. An unarmed fighter, or a weapon with no held body, SHALL pass the gate at once.

#### Scenario: A weapon knocked out of the hand
- **WHEN** an armed fighter's held weapon body is a metre from its grip as the attack's lead-in ends
- **THEN** the clip rate drops by 0.1 that tick, no window opens, and the gate opens on the first tick the weapon is back in the hand
