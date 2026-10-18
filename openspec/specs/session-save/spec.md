# session-save Specification

## Purpose
An Exarust save that restores the session and refuses the original's packages.

## Requirements

### Requirement: Round trip
A save SHALL restore the mode, the story level, the journal text, the chosen topics, the company's copper, roster and experience, and the focus.

#### Scenario: Career session
- **WHEN** a career that has advanced a level, written a journal line, chosen a topic, won a duel and set its focus is written and read back
- **THEN** those fields match and the topic cannot be chosen again

### Requirement: Refused file
A file that is not this save SHALL be refused, including one that is empty, cut short, or starts with the original save magic.

#### Scenario: Not ours
- **WHEN** the bytes are empty, cut short, or start with the original save magic
- **THEN** the read is an error
