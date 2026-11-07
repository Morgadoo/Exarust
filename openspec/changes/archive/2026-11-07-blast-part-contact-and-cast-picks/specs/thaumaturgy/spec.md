## MODIFIED Requirements

### Requirement: Barricade placement
A player's barricade SHALL stand at the cursor's ground pick (the hit on a floor mesh, else 15 cm back along the ray; nothing met places nothing), 10 cm up; a probe down of 210 cm that meets the floor SHALL move it there, with the wall's middle 100 cm above. It SHALL face away from the caster on the level and reach 300 cm each way; its placing sound's pitch SHALL be 0.8 + 0.4 times a roll.

#### Scenario: On the floor
- **WHEN** the cursor's pick meets (100, 0, 0) off a floor mesh along a ray straight down, the floor lies 20 cm below and the caster stands at (0, 50, 0) with a roll of 0.5
- **THEN** the middle is (100, 75, 0), the facing +x and the pitch 1.0

## ADDED Requirements

### Requirement: Force blast part contact
A thrown blast SHALL meet each part of another body in the part's frame: each live chunk whose reach `reach·scale + margin + 10` meets the part's node sphere and box SHALL be pushed out of every face of the node mesh it reaches to that reach, and the pushed chunk SHALL take the limb push. Each part met SHALL wake its body to 256 ticks and knock a character by that part's impulse.

#### Scenario: A chunk before a prop's face
- **WHEN** a chunk moving 5 cm toward a prop part's face stands 5 cm in front of it, with reach 4, mobility 1 and body push 0.5
- **THEN** it is pushed out to 20 cm from the face, moves 0.875 cm back from it, keeps 0.9 of its weight, and the part takes `5·(0.5/1.2)·0.2` along the move

### Requirement: Force blast limb velocity and push
The limb push of a blast's part contact SHALL take as the limb's velocity the part's particle motion this tick weighed by the chunk's barycentric weights over the particles, and its impulse SHALL move the particles by those weights and count toward the part's strain.

#### Scenario: A part at rest
- **WHEN** a chunk meets a part whose particles did not move this tick
- **THEN** the limb's velocity is zero and the push's share `f` is 1

### Requirement: Force blast simulated objects
A thrown blast SHALL stir each cloth whose node sphere and box its chunks reach: every chunk, spent or not, SHALL move each particle within `reach·scale + margin + 40` along its last move by `(1 − d²/r²)` times the setup's character push times 0.025 (narrowed once), and add `(1 − d²/r²)·0.0001` to the cloth's rustle envelope in double precision; the scale SHALL be the one the blast's last contact set. A cloth stirred SHALL wake to 256 ticks.

#### Scenario: A chunk past a banner
- **WHEN** a chunk of reach 1 at scale 2 moves 10 cm along a cloth whose particle lies 10 cm away
- **THEN** the particle moves `10·(1 − 100/1764)·0.8·0.025` along the move and the envelope rises by `(1 − 100/1764)·0.0001`

### Requirement: Cast ground pick
The throw, the long door and the summoning SHALL take the navigation unit's ground pick under the player's cursor: the throw growing the nodes with an object, the others every node. The throw SHALL take the body the pick meets when it may be thrown, and nothing when the pick meets the level. The long door's and the summoning's point SHALL be the hit as it stands on a floor mesh and 15 cm back along the ray otherwise.

#### Scenario: A summon on the floor
- **WHEN** the player summons with the cursor over the arena's floor
- **THEN** the demon stands on the floor where the cursor's ray meets it

#### Scenario: A long door against a wall
- **WHEN** the cursor's ray meets a wall 5 m away along a unit ray
- **THEN** the door's point lies 15 cm back along the ray from the hit

### Requirement: Barricade dismissal pick
A barricade cast while one runs SHALL take the nearest-mesh pick under the cursor (the ground plane's point when it meets nothing) and SHALL end the running barricade when the segment from the eye to that point meets its node's sphere and then its box.

#### Scenario: Cast again over the wall
- **WHEN** a barricade stands at the cursor's ground point and the player casts it again with the cursor unmoved
- **THEN** the running barricade ends and the cast answers 2
