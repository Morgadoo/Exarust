## ADDED Requirements

### Requirement: Keyframe track inspection
`exa-tool corpus tracks` SHALL list every keyframe track of every chunk tree of the installed packs, nested packs included: the file, the node and its parent, the play mode, the duration and every key (seconds, cm), then the numbers of chunk trees, tracks and undecodable tracks.

#### Scenario: Retail installation
- **WHEN** `exa-tool corpus tracks` runs against a retail installation
- **THEN** it lists one track, `Objects/uwa01 brkswtch [Box001] under Plane001: Once, 3 s, 4 keys`, out of 7653 chunk trees, none undecodable
