# Spec Delta

## Purpose

Turns the static mesh records of the original chunk-tree files into triangle
meshes, with material references, that Exarust and its tools can render or export.

## ADDED Requirements

### Requirement: Static mesh records
The system SHALL decode each static mesh record of a chunk-tree file into a named mesh with a local transform, vertex positions, texture coordinates, and normals when the record stores them.

#### Scenario: Grass prop
- **WHEN** `trn_grassa01.rfc` from `Resource.rpk` is decoded
- **THEN** two meshes named `Grass01 A` and `Grass01 B` are returned, each with matching position and texture-coordinate counts

### Requirement: Triangulated polygon groups
The system SHALL triangulate every polygon of a mesh and SHALL group triangles by the material name the polygon group references.

#### Scenario: Material groups
- **WHEN** a mesh with two polygon groups using different materials is decoded
- **THEN** two triangle groups are returned, each carrying its material name, and every index refers to an existing vertex

### Requirement: Units and axes
The system SHALL express decoded geometry in metres in a right-handed, Y-up frame, applying the conversion once at decode time.

#### Scenario: Known object size
- **WHEN** a standard door or table is decoded
- **THEN** its bounding box matches the real-world size of the object to within 10 %

### Requirement: Retail coverage
The system SHALL decode every static mesh record in Objlib, Objects, Components and the level-piece sets of Resource, and SHALL report unsupported records instead of failing the whole file.

#### Scenario: Corpus run
- **WHEN** every chunk tree in those packs is decoded
- **THEN** each static mesh record either decodes or is listed with the reason it is unsupported, and the decoded share is reported per pack

### Requirement: Malformed input
The system SHALL return an error, never panic, when a mesh record is truncated, declares impossible counts, or indexes outside its vertex streams.

#### Scenario: Out-of-range index
- **WHEN** a polygon references a vertex beyond the stream length
- **THEN** decoding that mesh fails with an error naming the mesh
