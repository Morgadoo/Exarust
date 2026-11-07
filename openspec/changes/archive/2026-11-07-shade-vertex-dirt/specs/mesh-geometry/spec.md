## ADDED Requirements

### Requirement: Stored vertex colours on dirt surfaces
The mesh build SHALL upload a mesh's stored colour stream, as four bytes a vertex in the group's vertex order, for every material group whose surface shows dirt mode 1 or 2, and SHALL upload no colour stream for any other group.

#### Scenario: A coloured barrel
- **WHEN** a model whose mesh stores colours draws one group with a grime material and one with a plain material
- **THEN** only the grime group's vertex buffer carries the colours, each vertex the stored bytes of the vertex it was built from

## MODIFIED Requirements

### Requirement: Vertex colour counts
`exa-tool corpus colours` SHALL report, per mesh pack, the meshes with stored vertex colours, the translucent and tinted ones, the vertices with alpha below 255, the meshes whose group materials select a dirt mode or the wound-blood mode, and the coloured meshes the running game shades with dirt mode 1 (grime) and 2 (wear), with examples.

#### Scenario: The character packs
- **WHEN** the command runs on the installation
- **THEN** CharExt, Characters and Components report no coloured meshes

#### Scenario: Objlib dirt
- **WHEN** the command runs on the 0.9.5 installation
- **THEN** Objlib reports 1187 coloured meshes shaded with grime and 596 with wear
