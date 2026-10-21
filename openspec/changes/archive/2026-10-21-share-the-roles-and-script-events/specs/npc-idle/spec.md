## MODIFIED Requirements

### Requirement: Init assignments
Applying roles SHALL start from Activity 1 and IdleActivity 0 and run each role's init script through the script interpreter, newest role first, so an older role wins; the script SHALL see the two activity slots and the held role's variables (0 for a variable the held role stores none of). Statements the interpreter reports an error for SHALL be returned with the role.

#### Scenario: Older wins
- **WHEN** Activity Wait is newer than Activity Patrol, a role's script adds a concern, and another names an unknown class
- **THEN** the character patrols, the concern statement runs without effect, and the unknown class is reported as "Script Error: Invalid class identifier."

#### Scenario: The shipped init scripts
- **WHEN** every init script of the installed `charroles.rdb` runs with its variables at 0 and at 9
- **THEN** the activities equal the scripts' plain assignments and nothing is reported
