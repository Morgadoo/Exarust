# Design

The combat score lives on its own component (`CombatScore`, required by every
fighter) so the damage routine can credit the striker while it holds the
struck fighter mutably. The clash and block notices are taken before any
other rule in their branch, as in the original, so a parry the clash ends
still counts.

The award reads the match's slot from the scene's level (the board's rank
column is the day slot) and the template's experience by id. A body is
credited when its record is found: the manager (id 0) or a roster member;
a fighter the tick removed is not.

Open questions: the player behaviour's notice is assumed to be the base
class's (no other override of the notice was found besides the AI's, which
calls the base first). The scene's body order is the query's, not the
scene list's.
