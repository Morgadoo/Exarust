# Design

The conversions follow the instructions, not the pseudo-code: the draws and
the experience award use `CVTSS2SI r64` and keep the low word, which
`arena_match::cvtss2si_low32` reproduces on top of the existing 64-bit
`convert`. A NaN or infinite weight therefore adds 0, not the 32-bit minimum.

The scene draw's walk starts its result index at 0, so a pick no running
weight reaches keeps the first team; with no team the original reads an
unset slot, and Exarust names none.

The health write-back keeps one word per member: the original writes the
record's two health words with the same value.

Reverted lane changes are reverted as new commits; the lane's history stays.
