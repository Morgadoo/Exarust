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
