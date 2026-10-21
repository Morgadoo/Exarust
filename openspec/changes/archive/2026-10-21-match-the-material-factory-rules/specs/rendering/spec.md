# Spec Delta

## MODIFIED Requirements

### Requirement: Shadow casters
The system SHALL cast shadows as the original does. A record in refraction mode casts; otherwise a surface casts only with opacity above 0.8 (double precision) and a loaded blend destination other than ONE. A record flagged by its material (extra flag `0x4`) casts none. Surfaces cast through the alpha-tested caster have their shadows cut; other surfaces cast whole.

#### Scenario: Cobwebs in torchlight
- **WHEN** a torch lights a wall hung with cobwebs
- **THEN** the cobwebs' shadows follow their strands instead of darkening the wall in solid sheets

#### Scenario: Refracting glass
- **WHEN** a record is in refraction mode with opacity 0
- **THEN** it casts a shadow


## ADDED Requirements

### Requirement: Late opaque rewrite
The system SHALL draw a blend word with destination nibble 3 opaque (source ONE, destination ZERO) when the record is in refraction mode, or when its opacity is above 0.99, its fall-off below 0.01 and its loaded flags lack the opacity layer bit, comparing in double precision.

#### Scenario: Faint glass kept blended
- **WHEN** a transparent record has opacity 0.5 and no refraction mode
- **THEN** it stays alpha-blended

#### Scenario: Opaque by opacity
- **WHEN** a transparent record has opacity 1, no fall-off and no opacity layer
- **THEN** it is drawn opaque


### Requirement: Diffuse premultiplication
The system SHALL scale a lit record's diffuse colour by its opacity when its masked mode bits are not refraction and its blend source, after the late rewrite, is ONE; an unlit record has no diffuse colour.

#### Scenario: Unlit record
- **WHEN** a record has only the emissive bit
- **THEN** its diffuse colour is zero


### Requirement: Factory feature word
The system SHALL take a record's feature word from its loaded flags masked by its type's plug-in, replacing a word with no low nibble by the emissive bit when the opacity is above zero (and drawing nothing otherwise). Its emissive colour SHALL double only when the word keeps the emissive bit, the lit or specular bits and the emissive layer.

#### Scenario: Emissive doubling
- **WHEN** a type-0 record has the emissive, lit and emissive-layer bits
- **THEN** its emissive colour is doubled


### Requirement: Factory map binding
The system SHALL bind a diffuse map only for a lit record, an emissive map only with the emissive bit, a specular map only with `0x100004`, and a normal map only for a lit, specular, fall-off or reflection material word.

#### Scenario: Unlit diffuse layer
- **WHEN** a record has a diffuse layer but only the emissive bit
- **THEN** no diffuse map is bound

#### Scenario: Type-2 record
- **WHEN** a type-2 record has emissive, specular and normal layers
- **THEN** none of those maps is bound
