## ADDED Requirements

### Requirement: Hire page records
The loadout form opened on the hire page SHALL take the constructor's hire-mode records before the layout: Previous and Next 0x70 wide with Next at x 0x78, Dismiss, the rank row and the shelf and box buttons hidden, and HIRE, the fee and upkeep captions and coins and the profession line shown.

#### Scenario: The hire page opens
- **WHEN** the hub opens the loadout form on the hire list
- **THEN** no shown record overlaps the profession line

### Requirement: Hire page candidate and text
The hire page SHALL show the shown candidate, wearing and holding its hire-list outfit, in the form's preview, and the profession's description in a 0xec by 0x100 text box at y 0x20, four pixels from the form's right edge.

#### Scenario: A merchant candidate
- **WHEN** the hire page shows a merchant
- **THEN** the preview shows the merchant in its outfit and the box reads the merchants' description

### Requirement: Coin control baseline
A company window's coin control SHALL draw its counts on its own baseline, the record's middle with the main font's centring height, so the counts sit on their captions' row.

#### Scenario: Hire fee and upkeep
- **WHEN** the hire page shows a candidate's fee and upkeep
- **THEN** each value is drawn on its caption's row
