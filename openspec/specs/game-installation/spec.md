# game-installation Specification

## Purpose

Locates and opens the user's own Exanima installation so every other part of
Exarust can read the original data in place, without copying or modifying it.

## Requirements

### Requirement: Installation discovery
The system SHALL locate the installation from, in order: an explicit path given on the command line, the `EXANIMA_DIR` environment variable, then an `EXANIMA_DIR=` line in a `.env` file in the working directory.

#### Scenario: Explicit path wins
- **WHEN** a path is passed with `--game` and `EXANIMA_DIR` is also set
- **THEN** the explicit path is used

#### Scenario: Fallback to .env
- **WHEN** no path is passed, `EXANIMA_DIR` is unset, and `.env` contains `EXANIMA_DIR="/games/Exanima"`
- **THEN** `/games/Exanima` is used, with surrounding quotes removed

#### Scenario: Nothing configured
- **WHEN** no source provides a path
- **THEN** the program exits with a message pointing to `--game`, `EXANIMA_DIR` and `.env.example`

### Requirement: Project file
The system SHALL read `Exanima.rfp` and expose the project name, the resource pack, and the object, texture and material library lists it names.

#### Scenario: Retail project file
- **WHEN** the retail `Exanima.rfp` is opened
- **THEN** the project name is `Exanima`, the resource pack is `Resource`, and three material libraries are listed

#### Scenario: Corrupt project file
- **WHEN** the file has a wrong magic, is truncated, or has trailing bytes
- **THEN** opening fails with an error describing the problem and no partial project is returned

### Requirement: Pack mounting
The system SHALL mount every known pack present in the installation and SHALL refuse an installation that has no `Resource.rpk`.

#### Scenario: Retail installation
- **WHEN** a complete retail installation is opened
- **THEN** all 12 packs are mounted and can be listed by role

#### Scenario: Missing resource pack
- **WHEN** the folder has `Exanima.rfp` but no `Resource.rpk`
- **THEN** opening fails with an error naming the folder

### Requirement: Read-only access
The system SHALL never write to, rename or lock files inside the installation.

#### Scenario: Inspecting an installation
- **WHEN** any Exarust tool or the game opens an installation
- **THEN** the installation's files are byte-identical afterwards
