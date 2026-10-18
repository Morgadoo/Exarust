# story-progress Specification

## Purpose
Story mode: which level is loaded, a dialogue choice, and the journal page.

## Requirements

### Requirement: Campaign
A new story SHALL load exanima01. Each advance SHALL load the next level through exanima08, and a further advance SHALL leave exanima08 loaded.

#### Scenario: Eight levels
- **WHEN** a new story advances once for each later level, then once more
- **THEN** the loaded levels are exanima02 through exanima08, and the extra advance stays on exanima08

### Requirement: Dialogue choice
A topic SHALL offer its choices until one is picked. The pick SHALL be that choice's line, and the topic SHALL not be offered again.

#### Scenario: One reply
- **WHEN** the second choice of a topic is picked, then a choice of that topic is picked again
- **THEN** the first pick is that line and the later pick is refused

### Requirement: Journal
A new journal SHALL open on a blank page titled New Entry. Writing SHALL store the text on the open page.

#### Scenario: First page
- **WHEN** the journal is written
- **THEN** the open page is titled New Entry and holds that text
