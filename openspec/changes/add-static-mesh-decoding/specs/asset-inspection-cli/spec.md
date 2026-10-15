# Spec Delta

## ADDED Requirements

### Requirement: glTF export
`exa-tool export-gltf` SHALL write the static meshes of a chunk-tree entry to a binary glTF file, with one primitive per material group and textures resolved by name from the installation.

#### Scenario: Export a prop
- **WHEN** `exa-tool export-gltf Objlib afr_cornflag_a01 out.glb` runs
- **THEN** `out.glb` opens in Blender showing the textured prop at its real-world size

#### Scenario: Missing texture
- **WHEN** a material names a texture that no pack contains
- **THEN** the export still succeeds and the missing name is reported as a warning

### Requirement: Mesh corpus report
`exa-tool corpus meshes` SHALL decode every chunk tree in the installation and print, per pack, how many static mesh records decoded and the distinct reasons for those that did not.

#### Scenario: Progress meter
- **WHEN** `exa-tool corpus meshes` runs against a retail installation
- **THEN** it prints a per-pack table and exits zero even when some records are unsupported
