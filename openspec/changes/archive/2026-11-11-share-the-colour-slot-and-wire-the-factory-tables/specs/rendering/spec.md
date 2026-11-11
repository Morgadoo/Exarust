## MODIFIED Requirements

### Requirement: Factory map binding
The system SHALL bind a diffuse map only for a lit record, an emissive map only with the emissive bit, a specular map only with `0x100004`, and a normal map only for a lit, specular, fall-off or reflection material word.

#### Scenario: Unlit diffuse layer
- **WHEN** a record has a diffuse layer but only the emissive bit
- **THEN** no diffuse map is bound

#### Scenario: Type-2 record
- **WHEN** a type-2 record has emissive, specular and normal layers
- **THEN** none of those maps is bound

#### Scenario: Unlit record with both layers
- **WHEN** an unlit record keeps the diffuse and opacity bits and has both layers
- **THEN** neither a diffuse nor an opacity map is bound

## ADDED Requirements

### Requirement: Shared colour slot
A record's diffuse and opacity layers SHALL fill one colour slot, the later layer in the record winning. A lit record with the diffuse bit SHALL draw that image as its colour and, with the opacity bit, as its opacity too; without the diffuse bit the opacity bit SHALL bind it as the opacity alone.

#### Scenario: An awning with two images
- **WHEN** a lit record names a diffuse image and then a different opacity image
- **THEN** both its colour and its opacity come from the opacity image

### Requirement: Colour slot load mode
The colour slot SHALL load a diffuse layer's image skipping the LOD bias when the record has a secondary colour, and an opacity layer's image in plain colour mode.

#### Scenario: Opacity after diffuse
- **WHEN** a record with a secondary colour has a diffuse and then an opacity layer
- **THEN** the slot's image takes the LOD bias

### Requirement: Factory start-up tables
The record build SHALL read the type table and factory list as the factory's start-up hook leaves them: the five built-in types and one factory record, the default builder serving plug-in mask bits 1 and 4. A record whose plug-in mask meets no factory record SHALL get no render state.

#### Scenario: A foreign plug-in mask
- **WHEN** a lit record's plug-in mask is 0x20
- **THEN** its feature word is zero and it has no state words

### Requirement: Loader render-state word
The record loader's render-state word SHALL keep the record word's 0x20 and add 1; 2 for a lit record or a positive fall-off; 0x100 for extra flags 0x4002; 2 and 0x100 for type 2; 4 with a normal layer; 0x100 with a dirt layer; and 0x10 when the loaded flags have any of 0xFEF0000.

#### Scenario: A lit record with a normal layer
- **WHEN** a lit type-1 record has a normal and a specular layer
- **THEN** its render-state word is 7
