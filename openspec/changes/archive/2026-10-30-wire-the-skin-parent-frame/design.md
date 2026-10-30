# Design

## Parent frame

The original keeps a stack of frames; each part's pass pushes its groups'
centres before recursing into its skinned children and pops after. A
child reads only the top frame, so in Exarust the frame is passed down
the recursion instead (`SkinPass::visit(index, incoming)`), with no
separate stack object. `skin::GroupStack` remains as the tested model of
the push limits.

The reader compares the child's group centre after the child node's
local transform, `(x·r0 + y·r1) + (z·r2 + t)` per component, with the
parent's centres (the parent's space, centimetres). Exarust stores node
translations in metres, so `t` is scaled back by 100; a translation that
does not survive the metre round trip may differ in the last bit.

A shipped skin stream makes the builder return early with its group
count cleared, and the pass still pushes, so the children see an empty
frame. The two group-list cleanups free the builder's groups (their
vertex lists and per-group arrays); they do not touch the stack and have
no Exarust counterpart beyond `Vec` drops.

## Condition

The three bytes are converted in the order damage, dirt, quality, each as
a double multiply narrowed to float; the remaps then run in float. The
tuple keeps Exarust's quality-first order.
