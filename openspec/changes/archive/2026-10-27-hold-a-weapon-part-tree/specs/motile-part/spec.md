# Spec Delta

## ADDED Requirements

### Requirement: Held weapon part-tree mass
When a weapon is equipped, its model's rigid nodes SHALL form a part tree that is redistributed in two passes, parent before children, with the weapon's table mass as the scale; the root part's live mass word SHALL then be divided by the holder's size while its inverse word keeps the distributed value.

#### Scenario: Two parts
- **WHEN** parts of mass 3 and 1 are redistributed with scale 0.75 for a holder of size 1.25
- **THEN** their inverse words are 1 / (0.375 × 0.75) and 1 / (0.625 × 0.75), and the root's live mass is its inverse divided by 1.25

### Requirement: Weapon length walk
The armed weapon length SHALL add the tops of the weapon's node and of each first-child node down the chain, stopping at the first node that is not rigid.

#### Scenario: Plain node between rigid ones
- **WHEN** the chain is rigid 50, plain 5, rigid 20
- **THEN** the length is 50

### Requirement: Joining the world
A part without foot contacts SHALL gain level contacts when it joins the world; a part with foot contacts SHALL lose level contacts and its dynamic bit, and each of its children SHALL lose its dynamic bit only.

#### Scenario: Lower leg and foot
- **WHEN** a fresh limb with foot contacts and its child join the world
- **THEN** the leg has neither level contacts nor the dynamic bit, and the child has only its family bits

### Requirement: Recent-impact counter order
Each part's recent-impact counter SHALL fall by one (down to 1) before the tick's contacts are handled.

#### Scenario: Struck once
- **WHEN** a limb is struck once in a tick
- **THEN** its counter reads 2 after that tick

### Requirement: Leaving part's blood spot
A part that is not a limb and holds a 1-based blood spot SHALL release it and take 0 when it leaves the world; a limb SHALL keep its spot.

#### Scenario: Weapon and limb
- **WHEN** a limb and then a weapon leave holding spot 1
- **THEN** the limb keeps 1 and the weapon releases the spot and takes 0
