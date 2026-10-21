//! The arena's random outfits: the pool picks that dress a generated
//! character or a hired candidate.
//!
//! The rules are the original's item-pool machinery: rank windows, the
//! weight a low-ranked entry keeps, the masked bonus check, the weighted
//! pick from a stepped roll, and the two-pass pick that refuses a piece
//! already worn. Evidence: the arena's outfit generation routines, read
//! from `Exanima.exe` 0.9.5 (see the arena research notes).

// The pool engine is filled in by the outfit change.
