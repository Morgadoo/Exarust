# Design

The body changes are in the existing kernel, so no new state is added. The edge length is one helper (`edge_length`) used for the rest and the current lengths, as the original computes its stored rest lengths with the same distance helper. The step keeps two statements so the two roundings stay separate. Sleep empties `moved` and `smoothed`; the original's third buffer (a previous-motion copy) has no counterpart here.

The volume's neighbour table stays a single 26-entry constant. The pass had read the loops' `compare with 25` as a 25-entry bound; the loops increment first and compare after the body, so the last index is 25. The shipped lists fitting only with 26 agrees.
