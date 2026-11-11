//! The Free Pascal run-time library's random generator, as the shipped game
//! links it: a Mersenne twister (MT19937) behind the `RandSeed` variable.
//!
//! Engine code calls three entry points of it, apart from the engine's own
//! LCG: `Random` (a double in `[0, 1)`, about 135 call sites: particles,
//! doors, thaumaturgy effects, the main menu, player controls), `Random(l)`
//! for a 32-bit whole number (tendril emitters, electricity, the divination
//! cards, the terrain manager, the object editor) and `Random(l)` for a
//! 64-bit one (fire effects). `Randomize` stores the tick count in
//! `RandSeed`; particle systems call it when they set up, so in the shipped
//! game the stream is clock-seeded once one exists. A few engine routines
//! store a fixed or per-object value in `RandSeed` (the item container,
//! the item description, the character textures). The next draw anywhere
//! then reseeds the twister from it.
//!
//! [`Mt19937`] is one stream seeded once, which is what every Exarust
//! subsystem keeps for itself. [`RtlRandom`] models the run-time library's
//! single process-wide generator with its `RandSeed` rule: a draw reseeds
//! when `RandSeed` differs from the value it left there, and a reseed stores
//! the complement of the seed back into `RandSeed`.

/// Words of state.
const N: usize = 624;
/// The twist offset.
const M: usize = 397;

/// The initialisation multiplier of the seeding loop.
const SEED_MULTIPLIER: u32 = 1_812_433_253;

/// 2⁻³², the scale of the unit draw.
const UNIT_SCALE: f64 = 2.328_306_436_538_696_3e-10;

fn seed_state(state: &mut [u32; N], seed: u32) {
    state[0] = seed;
    for i in 1..N {
        state[i] = SEED_MULTIPLIER
            .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
            .wrapping_add(i as u32);
    }
}

fn twist(state: &mut [u32; N]) {
    for i in 0..N {
        let y = (state[i] & 0x8000_0000) | (state[(i + 1) % N] & 0x7fff_ffff);
        let mut next = state[(i + M) % N] ^ (y >> 1);
        if y & 1 != 0 {
            next ^= 0x9908_b0df;
        }
        state[i] = next;
    }
}

fn temper(mut y: u32) -> u32 {
    y ^= y >> 11;
    y ^= (y << 7) & 0x9d2c_5680;
    y ^= (y << 15) & 0xefc6_0000;
    y ^= y >> 18;
    y
}

/// `Random(l)` for a 32-bit `l`: a negative `l` is first raised by one, and
/// the draw times `l` as a signed 64-bit product keeps its high word. So
/// `l > 0` gives `0..l` and `l < 0` gives `l+1..=0`.
pub fn scale_below(draw: u32, l: i32) -> i32 {
    let l = if l < 0 { l + 1 } else { l };
    ((i64::from(draw) * i64::from(l)) >> 32) as i32
}

/// `Random(l)` for a 64-bit `l`: the first draw is the low word, the second,
/// its top bit cleared, the high word, and the result is that value's signed
/// remainder by `l`. `l = 0` gives 0.
pub fn scale_below_i64(low: u32, high: u32, l: i64) -> i64 {
    let value = ((i64::from(high) << 32) & i64::MAX) | i64::from(low);
    if l == 0 { 0 } else { value.wrapping_rem(l) }
}

/// `Random` as a double: the draw times 2⁻³².
pub fn scale_unit(draw: u32) -> f64 {
    f64::from(draw) * UNIT_SCALE
}

/// One twister stream, seeded once (the run-time library's generator right
/// after `RandSeed` was set to `seed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mt19937 {
    state: Box<[u32; N]>,
    index: usize,
}

impl Mt19937 {
    /// Seeds the stream with the standard MT19937 initialisation.
    pub fn new(seed: u32) -> Self {
        let mut state = Box::new([0u32; N]);
        seed_state(&mut state, seed);
        Self { state, index: N }
    }

    /// The next tempered 32-bit output.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            twist(&mut self.state);
            self.index = 0;
        }
        let y = self.state[self.index];
        self.index += 1;
        temper(y)
    }

    /// `Random`: a unit draw in `[0, 1)`, `next_u32() · 2⁻³²`.
    pub fn unit(&mut self) -> f64 {
        scale_unit(self.next_u32())
    }

    /// `Random(l)` for a 32-bit `l` (see [`scale_below`]).
    pub fn random(&mut self, l: i32) -> i32 {
        scale_below(self.next_u32(), l)
    }

    /// `Random(l)` for a 64-bit `l` (see [`scale_below_i64`]): two draws.
    pub fn random_i64(&mut self, l: i64) -> i64 {
        let low = self.next_u32();
        let high = self.next_u32();
        scale_below_i64(low, high, l)
    }
}

/// The run-time library's process-wide generator with its `RandSeed`
/// variable. It starts with `RandSeed` 0, the value it last left there 0,
/// and the word index one past the end, so the first draw seeds from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtlRandom {
    /// `RandSeed`. Storing a new value makes the next draw reseed from it.
    pub rand_seed: u32,
    /// The value the last reseed left in `RandSeed`.
    old_rand_seed: u32,
    /// The index of the next word, kept as the original does: it counts on
    /// past the end until a draw twists the state.
    mti: u32,
    state: Box<[u32; N]>,
}

impl Default for RtlRandom {
    fn default() -> Self {
        Self {
            rand_seed: 0,
            old_rand_seed: 0,
            mti: N as u32 + 1,
            state: Box::new([0; N]),
        }
    }
}

impl RtlRandom {
    /// `Randomize`: stores the tick count in `RandSeed`.
    pub fn randomize(&mut self, tick_count: u32) {
        self.rand_seed = tick_count;
    }

    /// The next tempered 32-bit output. The index is read and then
    /// advanced. A changed `RandSeed`, or an index past the end, reseeds
    /// from `RandSeed`, stores its complement there and twists. An index at
    /// the end twists and restarts at the first word.
    pub fn next_u32(&mut self) -> u32 {
        let mut index = self.mti;
        self.mti = self.mti.wrapping_add(1);
        if self.rand_seed != self.old_rand_seed || index > N as u32 {
            seed_state(&mut self.state, self.rand_seed);
            self.rand_seed = !self.rand_seed;
            self.old_rand_seed = self.rand_seed;
            index = N as u32;
        }
        if index == N as u32 {
            twist(&mut self.state);
            index = 0;
            self.mti = 1;
        }
        temper(self.state[index as usize])
    }

    /// `Random`: a unit draw in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        scale_unit(self.next_u32())
    }

    /// `Random(l)` for a 32-bit `l`.
    pub fn random(&mut self, l: i32) -> i32 {
        scale_below(self.next_u32(), l)
    }

    /// `Random(l)` for a 64-bit `l`: two draws.
    pub fn random_i64(&mut self, l: i64) -> i64 {
        let low = self.next_u32();
        let high = self.next_u32();
        scale_below_i64(low, high, l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mt19937_known_vector() {
        let mut mt = Mt19937::new(5489);
        assert_eq!(mt.next_u32(), 3_499_211_612);
        assert_eq!(mt.next_u32(), 581_869_302);
        assert_eq!(mt.next_u32(), 3_890_346_734);
        assert_eq!(mt.next_u32(), 3_586_334_585);
        assert_eq!(mt.next_u32(), 545_404_204);
    }

    #[test]
    fn the_first_draws_seed_from_zero() {
        let mut rtl = RtlRandom::default();
        assert_eq!(rtl.next_u32(), 2_357_136_044);
        assert_eq!(rtl.rand_seed, 0xFFFF_FFFF);
        assert_eq!(rtl.next_u32(), 2_546_248_239);
        assert_eq!(rtl.next_u32(), 3_071_714_933);
        let mut mt = Mt19937::new(0);
        assert_eq!(
            [mt.next_u32(), mt.next_u32(), mt.next_u32()],
            [2_357_136_044, 2_546_248_239, 3_071_714_933]
        );
    }

    #[test]
    fn storing_rand_seed_restarts_the_stream() {
        let mut rtl = RtlRandom::default();
        rtl.next_u32();
        rtl.next_u32();
        // The item container stores a fixed seed; the next draw reseeds.
        rtl.rand_seed = 5489;
        assert_eq!(rtl.next_u32(), 3_499_211_612);
        assert_eq!(rtl.rand_seed, !5489);
        assert_eq!(rtl.next_u32(), 581_869_302);
        // Storing the same seed again restarts it again.
        rtl.rand_seed = 5489;
        assert_eq!(rtl.next_u32(), 3_499_211_612);
    }

    #[test]
    fn storing_the_complement_left_behind_does_not_reseed() {
        let mut rtl = RtlRandom {
            rand_seed: 5489,
            ..Default::default()
        };
        assert_eq!(rtl.next_u32(), 3_499_211_612);
        rtl.rand_seed = !5489;
        assert_eq!(rtl.next_u32(), 581_869_302);
    }

    #[test]
    fn the_word_index_wraps_after_624_draws() {
        let mut rtl = RtlRandom::default();
        let mut mt = Mt19937::new(0);
        for _ in 0..1300 {
            assert_eq!(rtl.next_u32(), mt.next_u32());
        }
    }

    #[test]
    fn randomize_stores_the_tick_count() {
        let mut rtl = RtlRandom::default();
        rtl.randomize(123_456);
        let mut mt = Mt19937::new(123_456);
        assert_eq!(rtl.next_u32(), mt.next_u32());
        assert_eq!(rtl.rand_seed, !123_456);
    }

    #[test]
    fn random_below_keeps_the_high_word() {
        // 3 499 211 612 · 22 = 76 982 655 464 → high word 17.
        assert_eq!(scale_below(3_499_211_612, 22), 17);
        assert_eq!(scale_below(0, 22), 0);
        assert_eq!(scale_below(u32::MAX, 22), 21);
        // A negative l is raised by one first: −10 draws in −8..=0.
        assert_eq!(scale_below(3_499_211_612, -10), -8);
        assert_eq!(scale_below(0, -10), 0);
        assert_eq!(scale_below(u32::MAX, -10), -9);
        assert_eq!(scale_below(u32::MAX, -1), 0);
        assert_eq!(scale_below(u32::MAX, 0), 0);
        let mut mt = Mt19937::new(5489);
        assert_eq!(mt.random(22), 17);
    }

    #[test]
    fn random_below_i64_joins_two_draws() {
        // High word 581 869 302 (top bit clear), low 3 499 211 612.
        let value = (581_869_302i64 << 32) | 3_499_211_612;
        assert_eq!(
            scale_below_i64(3_499_211_612, 581_869_302, 1000),
            value % 1000
        );
        // The second draw's top bit is cleared.
        assert_eq!(scale_below_i64(7, 0x8000_0000, i64::MAX), 7);
        assert_eq!(scale_below_i64(7, 1, 0), 0);
        // A negative divisor keeps the remainder's sign (non-negative here).
        assert_eq!(scale_below_i64(10, 0, -3), 1);
        let mut mt = Mt19937::new(5489);
        assert_eq!(mt.random_i64(1000), value % 1000);
    }

    #[test]
    fn the_unit_draw_scales_by_two_to_the_minus_32() {
        assert_eq!(scale_unit(0), 0.0);
        assert_eq!(scale_unit(0x8000_0000), 0.5);
        assert_eq!(scale_unit(u32::MAX), 1.0 - 2f64.powi(-32));
        let mut mt = Mt19937::new(5489);
        assert_eq!(mt.unit(), 3_499_211_612.0 / 4_294_967_296.0);
    }
}
