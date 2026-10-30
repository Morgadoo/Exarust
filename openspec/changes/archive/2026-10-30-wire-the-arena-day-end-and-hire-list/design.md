# Design

The run-time library's float random (the level draw of a trainer or
combatant candidate and the boards' rank rolls) is a Mersenne twister apart
from the engine's LCG; the random-NPC pass and the role draws step the LCG.
The career therefore holds one `arena::Mt19937` that the three callers read
in the original's call order, and the record pass starts the LCG from the
word the role draws left. The original seeds the twister once from the
clock; a default career seeds it from a fixed word.

The original stores the day's and the lifetime coin values as coin words.
Exarust keeps plain copper in `Company` and packs and unpacks at the save
tables, with the same 16-bit gold truncation as the coin setter.

The week's company field lives once, in `ArenaCareer::rivals`; the board
draw writes it, the board's winners table and the match launch read it.

The restock waits one frame for the live shop world (`hub_restock`); the
other day-end steps run in the button's handler in the original's order.

Lane commits are kept; the review adds commits on top.
