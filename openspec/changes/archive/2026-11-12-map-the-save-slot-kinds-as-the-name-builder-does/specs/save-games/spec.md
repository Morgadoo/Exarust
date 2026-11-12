## ADDED Requirements

### Requirement: Slot kind numbers
The slot kinds SHALL be the name builder's: kind 1 SHALL name the save (`.rsg`) and kind 2 the checkpoint (`.rcp`).

#### Scenario: The extension table
- **WHEN** the names of slot 1's two kinds are built with the story prefix
- **THEN** kind 1 is `Exanima001.rsg` and kind 2 is `Exanima001.rcp`

### Requirement: Character screen select kinds
The saves' character screen's select SHALL answer availability bit 2 when the slot's checkpoint opens and bit 1 when its save opens. It SHALL read the checkpoint when it opens and either kind 2 was asked or the save does not open, else the save when it opens.

#### Scenario: Both files
- **WHEN** a slot has a save and a checkpoint and is selected with kind 1, then with kind 2
- **THEN** the availability is 3 both times, the first read is the save and the second the checkpoint

#### Scenario: One file
- **WHEN** a slot has only its checkpoint and is selected with kind 1
- **THEN** the availability is 2 and the checkpoint is read

### Requirement: Continue kind
The saves' screen SHALL open at kind 2 when the first slot's availability is exactly 2, else at kind 1. Its CONTINUE SHALL read the checkpoint first only for kind 2.

#### Scenario: A fresh screen over both files
- **WHEN** the screen opens over a slot with a save and a checkpoint and CONTINUE is chosen
- **THEN** the save is loaded

#### Scenario: The toggle
- **WHEN** the toggle is used on that screen and CONTINUE is chosen
- **THEN** the checkpoint is loaded and copied over the save

### Requirement: Reset to portal file
The reset to the portal SHALL delete the file the select read, so with both files and kind 1 it SHALL delete the save and keep the slot listed for its checkpoint.

#### Scenario: Yes over both files
- **WHEN** Yes is chosen on a fresh screen over a slot with a save and a checkpoint
- **THEN** the save file is gone, the checkpoint stays and the slot stays listed and current
