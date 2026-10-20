## ADDED Requirements

### Requirement: Morph remap rebuild
The morph remap rebuild SHALL replace the map with `input[map[i]]`. For every part whose flags are clear it SHALL gather each element by `input[j]` over the part's stride and count and replace the part's bytes; a part with flags set SHALL be left alone.

#### Scenario: Composed map and gathered parts
- **WHEN** the map is (1, 0), the input is (1, 0) and a part has one-byte components in pairs with bytes 10, 11, 20, 21
- **THEN** the map is (0, 1) and the part's bytes are 20, 21, 10, 11
- **WHEN** another part has its flags set
- **THEN** its bytes are unchanged

### Requirement: Morph remap refresh
The rebuild SHALL always rebuild the arrays, indices and vertices. With bit 0 set it SHALL also purge, mark the owner dirty, stamp the version, and re-bind the bound object when one exists.

#### Scenario: Refresh bit set and clear
- **WHEN** bit 0 is set and a bound object exists
- **THEN** the rebuild purges, marks the owner dirty, stamps the version and re-binds
- **WHEN** bit 0 is clear
- **THEN** only the arrays, indices and vertices are rebuilt
