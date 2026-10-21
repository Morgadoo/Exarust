# Spec Delta

## MODIFIED Requirements

### Requirement: Duellist AI
The system SHALL drive an opponent from the start of the fight with the NPC combat state, once per body tick: it enters combat in the Defence stance with its eagerness at a half, turns to the state's heading, steps to the places its footwork search chooses and starts the blows it chooses, through the player's animation and attack paths. On guard it SHALL stand in Cmbt - Idle when fresh and in the Defence stance when tired; its reach SHALL be 60 cm plus its weapon's sweet spot, or 50 cm unarmed.

#### Scenario: Opponent engages
- **WHEN** `exarust play arena --opponent` starts
- **THEN** the opponent closes in and attacks the player

#### Scenario: Stepping in and striking
- **WHEN** an unarmed duellist faces its target 5 m away, and another faces its target 3 m away, for up to 1500 body ticks each
- **THEN** the first walks toward its target in the combat walk, and the second strikes from where it stands, its attack ending in its stance, while a duellist under eight tenths of its energy never strikes

#### Scenario: Far and near
- **WHEN** `exarust --still-opponent --at 0,0,6 play arenao1` runs, then `--at 0,0,1.7`
- **THEN** the fresh duellist stands in Cmbt - Idle both 8 m and 3.7 m from the player, facing it
