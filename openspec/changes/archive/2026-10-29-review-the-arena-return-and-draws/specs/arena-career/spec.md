## MODIFIED Requirements

### Requirement: Experience scale
The experience pool SHALL be the template amount times one plus a quarter of the tier, converted once to the nearest even integer, times the share of opponent health dealt (every body's in the Fray), converted again, and half again (truncated) when a trainer is hired. That pool SHALL be split among the company's combatants by their contribution counts, each share the pool times the contribution over the total divided in double and stored as single, converted to the nearest even integer.

#### Scenario: Two fighters
- **WHEN** two combatants win a duel at tier 0 after dealing all the health, with contributions 1 and 3, then a trainer is hired and they win a duel at tier 4 after dealing all the health, with equal contributions
- **THEN** the first result credits 250 and 750, and the second adds 1500 to each

#### Scenario: Unrounded tier factor
- **WHEN** a tier 3 match of 5000 experience ends with the opponents missing 130 of 200 health and contributions 2 and 3
- **THEN** the pool is 5688, the shares are 2275 and 3413, a trainer makes the pool 8532, and the Fray's 160 of 400 makes it 3500

## ADDED Requirements

### Requirement: Weighted draw conversion
Both weighted team draws SHALL convert each weight with the 64-bit conversion (ties to even) and keep its low 32 bits, so a NaN or infinite weight SHALL add 0. A scene pick that no running weight reaches SHALL keep the first team.

#### Scenario: Out-of-range weights
- **WHEN** NaN, infinity, 2^31 and −3e9 are converted
- **THEN** they give 0, 0, the 32-bit minimum and 1294967296

### Requirement: Return health write-back
The return to the hub SHALL write each company body's health word into its record: participant 0 into the manager, participant n into roster member n − 1, and nothing for a participant the roster no longer has. Company bodies SHALL carry the participant id the numbering pass gives.

#### Scenario: Manager and roster
- **WHEN** the manager returns at half health, roster member 2 at none, and participant 9 at a tenth
- **THEN** the manager's word is 32768, member 2's is 0, member 1 keeps 65535, and two words are written

### Requirement: Rank-up kit copy
A lesson that raises a fighter's level sum to an even sum from 2 through 10 SHALL copy kit sum/2 − 1 over kit sum/2.

#### Scenario: Crossing a rank
- **WHEN** a lesson takes a fighter from 9600 to 10000 points in one skill with kit 0 holding word 9
- **THEN** kit 1 holds word 9

### Requirement: Rival week company
A rival champion of a week SHALL record the company at its own team's field slot plus 2, not the first tallied team's.

#### Scenario: Champion tallied second
- **WHEN** the rows are [3,2,2,2,4,5] with field company 15 at slot 0
- **THEN** no player champion is recorded and the week holds company 15+2
