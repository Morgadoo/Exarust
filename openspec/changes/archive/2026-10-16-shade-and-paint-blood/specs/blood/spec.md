## ADDED Requirements

### Requirement: Bleeding races
The system SHALL read a race's flags from the word at `0x18` of its `races.rdb` record and treat bit 0 as bleeding, for every record of the installed `races.rdb`: Human, Ogre, Sir, Fael, Ghoul, Ghoul H, Gobbler, Manipulator, the harvester and the demon bleed; Skeleton, Ancient, Wraith and the five golems do not.

#### Scenario: Human
- **WHEN** the installed `races.rdb` is read
- **THEN** the Human record's flags are `0x0F` and it bleeds

#### Scenario: Golem
- **WHEN** the installed `races.rdb` is read
- **THEN** the Golem Guard record's flags are `0x1030` and it does not bleed

### Requirement: Blood channel
The system SHALL keep wound blood as one byte per vertex, the alpha of the mesh's colour stream: 255 clean, lowered by wounds and never raised. A mesh without a colour stream SHALL count as white. The first wound that lowers a value of a mesh shared with other characters SHALL give the struck character its own copy; later wounds SHALL paint that copy in place.

#### Scenario: Shared braies
- **WHEN** one of two characters wearing the same braies is wounded on the thigh
- **THEN** only its braies show blood, and the other's meshes stay the shared, clean ones

#### Scenario: Clean characters
- **WHEN** no wound has struck a character
- **THEN** none of its meshes is copied and it looks as it did without blood

### Requirement: Wound targets
A wound SHALL paint its character's skinned meshes (the body and every skinned garment) and every other mesh under its head part (hair, helmets, hoods), as the original paints its merged skinned mesh and the head's meshes. It SHALL NOT paint held items, rigid items on other parts, or body parts hidden under apparel.

#### Scenario: Helmet
- **WHEN** a wound reaches the head of a duellist wearing `helm_b`
- **THEN** the helmet takes blood as the skin does

#### Scenario: Held sword
- **WHEN** a wound's reach covers the sword held across the chest
- **THEN** the sword's mesh is left as it is

### Requirement: Wound painting
A wound with contact point P, travel D and radius r SHALL be painted around c = P − D/2 on the vertices' current skinned positions: e = |d|²(1 − cos θ) ahead of c along D and |d|² behind it (d from c); where e ≤ r², f = 1 − e/r², g = f + (1 − f)·u with u uniform in [0, 1), and the value 255·(1 − f·g²), rounded to the nearest byte, written only when lower.

#### Scenario: Soaked centre
- **WHEN** a vertex lies on the wound's axis
- **THEN** its value becomes 0

#### Scenario: Speckled edge
- **WHEN** vertices lie at f = ½ and at f = ¼
- **THEN** their values fall in 128–223 and 191–251

#### Scenario: Smear
- **WHEN** the striking point travelled 30 cm down
- **THEN** every vertex straight below the centre is soaked however far, and above it nothing beyond r is painted

### Requirement: Blood shading
Surfaces with blood SHALL show it where the vertex alpha α falls below the speckle `spatter01` (a, at UV×2, ×1 on hair): da = clamp((a − α)/a·k, 0, 1), k 1.3 on cloth, 1 on skin (α no lower than 0.02) and hair. The diffuse SHALL take T·s·(1 − 0.3·da) + (1 − da), clamped, T (0.6, 0.03, 0) (hair (0.45, 0.03, 0.02)), s 1 − da²/2 on skin, else 1.

#### Scenario: Thick blood
- **WHEN** da is 1
- **THEN** the diffuse colour is multiplied by (0.21, 0.0105, 0) on skin and (0.42, 0.021, 0) on cloth

#### Scenario: Lightly painted vertex
- **WHEN** a skin vertex's value is 128
- **THEN** blood shows only on the speckle's texels above 0.5, about 13 % of them, as separate droplets

### Requirement: Wet blood
Blood SHALL wet a surface: skin roughness SHALL lose da·(0.1 + 0.6·a) and stay at least 0.1; cloth roughness SHALL move toward 0.2 by da²·(0.5·a + 0.5), and hair roughness by da²·(0.7·a + 0.3).

#### Scenario: Soaked skin
- **WHEN** skin of roughness 0.6 is soaked (da 1) where the speckle is 0.5
- **THEN** its roughness is 0.2 and the key light leaves a sharp highlight on it

### Requirement: Materials with blood
The system SHALL give blood to the human skin (skin program), to painted and run-time apparel and factory items (cloth), to any record whose extra flags select dirt mode 4 (`0x7000` = `0x4000`) and that has a diffuse map, by type (1 cloth, 3 hair, 4 skin), and to a bleeding race's body-model materials whose names start with `s`. The ancient skin and eyes SHALL have none.

#### Scenario: Ogre
- **WHEN** an ogre body is loaded
- **THEN** its `skinogre` material shows blood as cloth, and `ogreeyemouth` none

#### Scenario: Ancient
- **WHEN** an ancient body is loaded
- **THEN** its skin shows no blood

#### Scenario: Braies
- **WHEN** a human wears the underwear
- **THEN** the painted braies show blood as cloth
