# asset-inspection-cli Specification

## Purpose

`exa-tool` lets developers inspect and extract the original data from their
own installation without writing code, as the primary instrument for format work.

## Requirements

### Requirement: Installation inventory
`exa-tool info` SHALL print the installation path, the project file contents, and each mounted pack with its entry count and total size.

#### Scenario: Retail install
- **WHEN** `exa-tool info` runs against a retail installation
- **THEN** 12 packs are printed with entry counts

### Requirement: Entry listing and statistics
`exa-tool ls` SHALL list a pack's or nested archive's entries with size and detected format, and `exa-tool stats` SHALL total entries and bytes per format.

#### Scenario: Nested listing
- **WHEN** `exa-tool ls Resource --inner common.rml` runs
- **THEN** the material library's entries are listed

### Requirement: Format inspection
`exa-tool chunks` SHALL print the chunk tree of a chunk-tree entry to a chosen depth, and `exa-tool rfi` SHALL print an image's header, format and mip count.

#### Scenario: Chunk tree
- **WHEN** `exa-tool chunks Resource trn_grassa01.rfc` runs
- **THEN** the material list and the mesh list with two meshes are printed

### Requirement: Image export
`exa-tool png` SHALL write the decoded base level of an image to a PNG file.

#### Scenario: Diffuse texture
- **WHEN** `exa-tool png Resource humandiff.rfi out.png` runs
- **THEN** a 1024x1024 RGBA PNG is written

### Requirement: Extraction
`exa-tool extract` SHALL write entries of a pack or nested archive to a directory, optionally filtered by a name substring, adding an extension from the detected format when the name has none.

#### Scenario: Filtered extraction
- **WHEN** `exa-tool extract Sound out --filter msc_` runs
- **THEN** only music entries are written, as `.wav` files

### Requirement: Clear failures
Every command SHALL exit with a non-zero status and a message naming the missing pack or entry when a lookup fails.

#### Scenario: Unknown entry
- **WHEN** `exa-tool rfi Textures does_not_exist` runs
- **THEN** it exits non-zero with a message naming the entry and pack

### Requirement: Race inspection
`exa-tool races` SHALL print every race of the installation's `races.rdb`: its id, name, body model, movesets and health, its flag words (whether clawed and regenerating red), its body parameters (clip-rate base, size, scale, mass divisor, support-pin factor, balance bonus, stance factor, arm relaxation, damping ratio), its stats (unarmed reach, swing turn, recovery, AI attack bonus, perception, held offset, footsteps, voice volume, attack voices) and its four gait rows.

#### Scenario: Retail races
- **WHEN** `exa-tool races` runs against a retail installation
- **THEN** 18 races are printed, Human with stance factor 0.5 and a walk thrust of 88.9 per second

#### Scenario: Ghoul
- **WHEN** `exa-tool races` runs against a retail installation
- **THEN** Ghoul is printed with flags `0x3` / `0x3`, kind `0x100` and clawed, clip-rate base 1.5, unarmed reach 1 m and footsteps `StpSft`

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

### Requirement: Body and mechanism inspection
`exa-tool bodies` SHALL print the rigid bodies and cloth markers of a chunk-tree entry's nodes (root or child, class, mobility, rest points, spheres, links with a slider's travel, sound slots, in cm); `exa-tool mechanisms` SHALL print the door and operative records of `objdb.rdb` or of a level's own database with the mechanism each gives its model; `exa-tool corpus bodies` SHALL count bodies, links, sliders and cloth markers per model pack.

#### Scenario: A lever
- **WHEN** `exa-tool bodies Objects "lever a01"` runs
- **THEN** one root body is printed with two axle pins and an arm link of 33.81 cm slack

#### Scenario: First level
- **WHEN** `exa-tool mechanisms exanima01` runs
- **THEN** 43 doors and 72 operative records are printed

#### Scenario: Corpus
- **WHEN** `exa-tool corpus bodies` runs
- **THEN** `Objects.rpk` shows 1 163 of 1 253 models with bodies, none undecodable, 69 with links, 65 hung from the world

### Requirement: Navigation grid inspection
`exa-tool nav <level>` SHALL build the level's navigation grid as the game does at load, without a window, and print its size, inputs, columns by span count and, per layer, its floor, passable and open-sky cells; `--png <file>` SHALL draw both layers side by side, a pixel per cell.

#### Scenario: exanima01
- **WHEN** `exa-tool nav exanima01 --png nav.png` runs against a retail installation
- **THEN** it prints 640 × 800 cells with 219292 lower floors and writes a 1284 × 800 picture of the dungeon's rooms, black over the rock between them
