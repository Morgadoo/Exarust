# Spec Delta

## Purpose

Shows the original game's models and levels inside the Bevy runtime, as the
first step from decoded data to a playable world.

## ADDED Requirements

### Requirement: Model view
`exarust model <pack> <entry>` SHALL display every static mesh of the entry with its textures, framed by a camera that can fly around it.

#### Scenario: Prop
- **WHEN** `exarust model Objlib afr_cornflag_a01` runs
- **THEN** the textured plant is visible at its real size with alpha-masked leaves

### Requirement: Level view
`exarust level <name>` SHALL display the level's own scene meshes and every prop placed from the object library at its stored transform, with lighting and a free-fly camera.

#### Scenario: Story level
- **WHEN** `exarust level exanima01` runs
- **THEN** the level's props appear in their stored arrangement and the camera can move through the level

### Requirement: Shared resources
The viewer SHALL load each distinct model, material and texture once, however many times it is placed.

#### Scenario: Repeated props
- **WHEN** a level places the same object hundreds of times
- **THEN** its meshes and textures are decoded and uploaded once

### Requirement: Missing data is not fatal
The viewer SHALL show what it can when a prop's object, a material or a texture is missing, and log what was skipped.

#### Scenario: Unknown object
- **WHEN** a prop names an object absent from the installation
- **THEN** the level still loads and the missing name is logged once
