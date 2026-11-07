## MODIFIED Requirements

### Requirement: Constant Displacement and the Summon
Constant Displacement's hum SHALL rise by the double 0.002 each control tick while its volume is under the double 0.12. Once its demon has arrived the summon SHALL hold it for 2000 ms, open the shell over it for 3000 ms while writing `1 − (deadline − now)/3000` as the caster's gain, then let the demon go and close the shell, and SHALL end 1000 ms later.

#### Scenario: The summon's fade
- **WHEN** a summon's demon arrives at 4 ms
- **THEN** the shell opens at 2004 ms, the gain is one half at 3504 ms, the demon is let go at 5004 ms and the summon is done at 6004 ms

## ADDED Requirements

### Requirement: The summoned demon's place
A summon SHALL bring in the character record `demon a01` at its place: the cursor's for the player, the Long Door's grid answer for the AI (an impossible answer ends the summon). The demon SHALL face the caster along the level line from its place, or the caster's own way when that line is 1 cm or shorter.

#### Scenario: A summon at the caster's feet
- **WHEN** the place lies 0.7 cm from the caster across the floor
- **THEN** the demon faces the caster's own way

### Requirement: The summoned demon's ties
The demon SHALL take role 33, and demon and caster SHALL each take familiarity 75 and relation role 28 toward the other. At its arrival the demon SHALL be held (its statue byte set) and the caster's power memory SHALL keep it; the AI SHALL never advise the summoning again, and nothing SHALL remove the demon later.

#### Scenario: The AI after its summon
- **WHEN** a caster's demon has arrived and later died
- **THEN** its summoning advice still refuses

### Requirement: The summon's phase table
The summon SHALL run its own phase table over the demon: a wait, the depth pass until 2800 ms staging the body, the depth and fade passes until 3200 ms handing it back to the scene, then the shown end. It SHALL start at its first entry with the body revealed and move to its second entry when the shell opens.

#### Scenario: The body comes in
- **WHEN** the shell opened 3000 ms ago
- **THEN** the demon's body is shown at one half

### Requirement: The demon's shell and tendrils
The demon's shell SHALL rest at nothing shown, open over 3000 ms and close over 1000 ms, at opacity 2 and a texture scale divided by the demon's size. The tendril image SHALL play on the demon from its arrival until it is let go.

#### Scenario: The shell over the demon
- **WHEN** the shell has been opening for 1500 ms
- **THEN** it is half shown
