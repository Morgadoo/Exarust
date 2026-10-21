## ADDED Requirements

### Requirement: Fixed match places
A one-on-one match (Duel, Challenger, Elimination, Reserve, Pugilism) SHALL place its opponent at the spawn spot plus (0, 5, −200) cm facing +z and the company's fighter at the spawn spot plus (0, 0, 200) facing −z. Pugilism's opponent SHALL have no gear tier.

#### Scenario: A duel on arenao3's spot
- **WHEN** a duel of level 3 and tier 2 is laid out at spawn (900, 0, 100), pugilism likewise, and template 99
- **THEN** the opponent of level 3 and tier 2 on team 2 with flags 0x12 stands at (900, 5, −100) facing +z, the company at (900, 0, 300) facing −z; pugilism's opponent has tier −1; template 99 lays out nobody

### Requirement: Ring places
A ring place SHALL turn a fraction of a turn (the single-precision turn times the fraction) into the direction (sin, 0, cos) of the engine's sine and cosine, stand at the spawn spot plus the direction times −350, and face along the direction. A fray's place SHALL use the index times the turn times a sixth.

#### Scenario: Half a turn and none
- **WHEN** fractions 0.5, 0, −0.05 and 0.05 are placed
- **THEN** 0.5 stands 350 behind the spot facing −z, 0 stands 350 ahead facing +z, and ∓0.05 stand mirrored about 108.16 to either side

### Requirement: Team match layouts
Doubles SHALL place opponents at ∓0.05 and the company at 0.55 and 0.45. Valiance SHALL place two opponents a level and a tier lower at ∓1/15 and the company at 0.5. Valour SHALL place three two lower at −1/12, 0 and 1/12; Captain the same with the middle one at the match's level and the company at 0.5, 7/12 and 5/12. Skirmish SHALL place three at the match's level and the company at 7/12, 0.5 and 5/12, and the Beast its beast at the fixed opponent place with the company the same.

#### Scenario: Lowered opponents
- **WHEN** Valiance, Valour, Captain, Skirmish, the Beast and Doubles are laid out at level 4 and tier 3
- **THEN** Valiance's opponents are level 3 tier 2, Valour's level 2 tier 1, Captain's middle opponent level 4 and its second entrant at 7/12, Skirmish's first entrant at 7/12 and second at 0.5, the beast at the opponent's place, and Doubles' first entrant at 0.55

### Requirement: Fray layout
A fray or a brawl SHALL place six fighters a sixth of a turn apart: the company's fighter fourth, each other an opponent in a free for all (flags 0x10) on team 1 shifted by its index plus 3.

#### Scenario: Six teams
- **WHEN** a fray and a brawl are laid out at the origin
- **THEN** both are the same six places, the fourth is the company's, the teams are 8, 16, 32, 128 and 256, and the first stands at (0, 0, −350)

### Requirement: Next round
A multi-round match SHALL bring a fresh opponent of its level and tier while no round is lost or its fighter still stands; otherwise the opponent SHALL stay and the entrant at the place of the losses SHALL come in.

#### Scenario: Fresh or reserve
- **WHEN** the next round follows no loss with the fighter dead, one loss with it standing, and two losses with it dead
- **THEN** the first two bring a fresh opponent and the third brings entrant 2

### Requirement: Running match layout
The running match SHALL take its rounds from its template (Elimination four extra rounds, Reserve two and three lives). In play mode `--arena-match <id>` SHALL put the player on the company's first place and a duellist on each opponent place; other entrants and the beast SHALL NOT be spawned.

#### Scenario: Template rounds
- **WHEN** templates 7, 5 and 1 begin a match
- **THEN** 7 has four extra rounds, 5 three lives, and 1 the single round
