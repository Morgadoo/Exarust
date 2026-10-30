## ADDED Requirements

### Requirement: Hub windows from their control tables
Manage Company, Company Records, Company Status, the loadout form and the dismiss and yes/no dialogs SHALL be built from their constructors' record arrays (18, 27, 12, 28, 3 and 3 records), laid out at unit 4, rescaled to the live unit and placed by the form's placement bits, and a record with a callback SHALL run the arena's rule for it when a press and release land on it.

#### Scenario: Manage Company at unit 4
- **WHEN** Manage Company opens on a 1280 × 720 screen at unit 4
- **THEN** the form is 708 × 468 at 286, 126, Manage sits at 40, 330 (120 × 28) and Recruit at 40, 370

#### Scenario: Dismissing a fighter
- **WHEN** the loadout form steps Next to an employee, Dismiss is pressed and the dialog's Dismiss is pressed
- **THEN** the dialog asks "Are you sure you want to dismiss " the name "?", the employee leaves the company and the list shows the manager again
