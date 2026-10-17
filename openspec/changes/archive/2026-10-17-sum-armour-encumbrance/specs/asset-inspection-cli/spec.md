## MODIFIED Requirements

### Requirement: Character inspection
`exa-tool characters <level>` SHALL print every record of a level's character database with where the level places it, and `exa-tool characters actors` every arena actor: name, version, race, sex, feral level, height, age, physique, pools, combat skill, experience, traits, voice, each skill with its learnt techniques, perks, Manoeuvring tier, Deflection, roles, worn and carried counts, saved state, powers, and the stats it spawns with, its encumbrance among them.

#### Scenario: First story level
- **WHEN** `exa-tool characters exanima01` runs against a retail installation
- **THEN** 42 records and 39 placements are printed, with Zombie01 feral 1 and holding the roles Corpse and Undead

#### Scenario: Arena actors
- **WHEN** `exa-tool characters actors` runs against a retail installation
- **THEN** 21 actors are printed, opponent16 with Close Combat 20000 (Remise, Feint, Fend, Riposte) and Armour 25000

#### Scenario: Spawn stats
- **WHEN** `exa-tool characters exanima01` and `exa-tool characters actors` run against a retail installation
- **THEN** Zombie01 spawns with most energy 0.9, no head gain and health 0.25 of 0.25, and every opponent with full health

#### Scenario: Encumbrance
- **WHEN** `exa-tool characters actors` runs against a retail installation
- **THEN** opponent07 spawns with encumbrance 0.475 and a step reach shortened by it, and the demon with none
