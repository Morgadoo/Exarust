//! Terrain blend rules: painted layers resolved, per vertex, into weights of
//! the palette's materials.
//!
//! Rules and materials form one node list (rules first). A rule reads one
//! per-vertex input selected by its kind and sends its weight to its first
//! child below `lo`, to its second above `hi`, and splits it linearly in
//! between. Weight the painted layers leave over goes to node 0. Evidence:
//! `docs/formats/TERRAIN.md`.

use exa_formats::terrain::Palette;

/// Side of the repeating noise maps, in terrain vertices (96 m).
pub const NOISE_SIDE: usize = 480;
/// Noise maps are generated from these seeds (inputs 3, 4 and 7).
const NOISE_SEEDS: [u32; 3] = [137, 476, 7359];
const NOISE_OCTAVES: u32 = 4;
/// Leftover weight below this is ignored.
const LEFTOVER_EPSILON: f32 = 0.001;
/// Deeper rule chains are treated as cycles.
const MAX_DEPTH: usize = 32;

/// Per-vertex values a rule can select by its kind.
pub const INPUTS: usize = 9;

#[derive(Debug, Clone, Copy)]
enum Node {
    Rule {
        kind: usize,
        lo: f32,
        hi: f32,
        a: Option<usize>,
        b: Option<usize>,
    },
    Material(usize),
}

/// A palette's rules and materials with child names resolved.
pub struct RuleTree {
    nodes: Vec<Node>,
    materials: usize,
}

impl RuleTree {
    pub fn new(palette: &Palette) -> Self {
        let mut nodes: Vec<Node> = palette
            .rules
            .iter()
            .map(|r| Node::Rule {
                kind: r.kind as usize,
                lo: r.lo,
                hi: r.hi,
                a: Self::node(palette, &r.a),
                b: Self::node(palette, &r.b),
            })
            .collect();
        nodes.extend((0..palette.materials.len()).map(Node::Material));
        Self {
            nodes,
            materials: palette.materials.len(),
        }
    }

    pub fn material_count(&self) -> usize {
        self.materials
    }

    /// Node of a layer or child name: rules shadow materials of the same name.
    pub fn node(palette: &Palette, name: &str) -> Option<usize> {
        let name = name.trim();
        palette
            .rules
            .iter()
            .position(|r| r.name.eq_ignore_ascii_case(name))
            .or_else(|| {
                palette
                    .materials
                    .iter()
                    .position(|m| m.name.eq_ignore_ascii_case(name))
                    .map(|m| palette.rules.len() + m)
            })
    }

    /// Material weights of one vertex. `layers` pairs each painted layer's
    /// node with its weight (0..=1); `out` has one slot per material.
    pub fn weigh(&self, layers: &[(usize, f32)], inputs: &[f32; INPUTS], out: &mut [f32]) {
        let mut painted = 0.0;
        for &(node, w) in layers {
            if w > 0.0 {
                painted += w;
                self.descend(node, w, inputs, out, 0);
            }
        }
        let leftover = 1.0 - painted;
        if leftover > LEFTOVER_EPSILON && !self.nodes.is_empty() {
            self.descend(0, leftover, inputs, out, 0);
        }
    }

    fn descend(&self, node: usize, w: f32, inputs: &[f32; INPUTS], out: &mut [f32], depth: usize) {
        match self.nodes.get(node) {
            Some(&Node::Material(m)) => out[m] += w,
            Some(&Node::Rule { kind, lo, hi, a, b }) if depth < MAX_DEPTH => {
                let v = inputs.get(kind).copied().unwrap_or(0.0);
                let t = if v <= lo {
                    0.0
                } else if v >= hi {
                    1.0
                } else {
                    (v - lo) / (hi - lo)
                };
                if t < 1.0
                    && let Some(a) = a
                {
                    self.descend(a, w * (1.0 - t), inputs, out, depth + 1);
                }
                if t > 0.0
                    && let Some(b) = b
                {
                    self.descend(b, w * t, inputs, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// The three repeating noise maps rules can read.
pub struct NoiseMaps {
    maps: [Vec<u8>; 3],
}

impl Default for NoiseMaps {
    fn default() -> Self {
        Self {
            maps: NOISE_SEEDS.map(noise_map),
        }
    }
}

impl NoiseMaps {
    /// Map `which` (0..3) at vertex (`i`, `j`) of tile (`tile_x`, `tile_y`),
    /// 0..=255. Adjacent tiles continue the map; it repeats every two tiles.
    pub fn sample(&self, which: usize, tile_x: u32, tile_y: u32, i: usize, j: usize) -> u8 {
        let half = NOISE_SIDE / 2;
        let x = ((tile_x as usize % 2) * half + i) % NOISE_SIDE;
        let z = ((tile_y as usize % 2) * half + j) % NOISE_SIDE;
        self.maps[which][z * NOISE_SIDE + x]
    }

    /// The rule inputs of one vertex.
    pub fn inputs(
        &self,
        height: f32,
        up: f32,
        tile_x: u32,
        tile_y: u32,
        i: usize,
        j: usize,
    ) -> [f32; INPUTS] {
        let a = ridge(self.sample(0, tile_x, tile_y, i, j)) as f32 / 255.0;
        let b = self.sample(1, tile_x, tile_y, i, j) as f32 / 255.0;
        let c = self.sample(2, tile_x, tile_y, i, j) as f32 / 255.0;
        [
            height,
            up * up,
            // 2: a map the original never fills.
            0.0,
            a,
            b,
            // 5: unused by every palette.
            0.0,
            // 6: an override map that is never set, so the first noise.
            a,
            c,
            0.5,
        ]
    }
}

/// Folds a noise value into a ridge that peaks at mid-range.
fn ridge(v: u8) -> u8 {
    let f = if v < 128 {
        v as f32 / 127.0
    } else {
        (255 - v) as f32 / 127.0
    };
    (f * f * 255.0).round().min(255.0) as u8
}

/// White noise from a linear congruential sequence, then four octaves of
/// progressively coarser copies of it averaged in, coarsest last.
fn noise_map(seed: u32) -> Vec<u8> {
    let n = NOISE_SIDE;
    let mut state = seed;
    let white: Vec<u8> = (0..n * n)
        .map(|_| {
            state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
            ((state as u64 * 255) >> 32) as u8
        })
        .collect();
    let mut out = white.clone();
    for octave in 1..=NOISE_OCTAVES {
        let g = n >> octave;
        let scale = (1u32 << octave) as f32;
        let cell = |x: usize| -> (usize, usize, f32, f32) {
            let c = ((x as f32 + 0.5) / scale).trunc();
            let f = x as f32 / scale - c;
            let c = c as usize;
            ((c + g - 1) % g, c % g, 0.5 - f, 0.5 + f)
        };
        for y in 0..n {
            let (y0, y1, wy0, wy1) = cell(y);
            for x in 0..n {
                let (x0, x1, wx0, wx1) = cell(x);
                let at = |cx: usize, cy: usize| white[cy * g + cx] as f32;
                let sample = (at(x0, y0) * wx0 + at(x1, y0) * wx1) * wy0
                    + (at(x0, y1) * wx0 + at(x1, y1) * wx1) * wy1;
                let p = &mut out[y * n + x];
                *p = ((*p as f32 + sample) / 2.0)
                    .round_ties_even()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// "Over" opacities of materials stacked in order: the first is opaque and
/// each later one covers what is below by its share of the weight so far.
pub fn over_alphas(weights: &[f32]) -> Vec<f32> {
    let mut sum = 0.0;
    weights
        .iter()
        .enumerate()
        .map(|(k, &w)| {
            sum += w;
            if k == 0 {
                1.0
            } else if sum > 0.0 {
                w / sum
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::terrain::{BlendRule, TerrainMaterial};

    fn rule(name: &str, kind: u32, a: &str, b: &str, lo: f32, hi: f32) -> BlendRule {
        BlendRule {
            name: name.into(),
            flags: 0,
            kind,
            a: a.into(),
            b: b.into(),
            lo,
            hi,
        }
    }

    fn material(name: &str) -> TerrainMaterial {
        TerrainMaterial {
            name: name.into(),
            flags: 0,
            textures: Default::default(),
            params: vec![],
        }
    }

    fn palette(rules: Vec<BlendRule>) -> Palette {
        Palette {
            name: "p".into(),
            materials: vec![material("Sand"), material("Rock"), material("Grass")],
            rules,
            plants: vec![],
        }
    }

    fn inputs(height: f32, up: f32) -> [f32; INPUTS] {
        let mut i = [0.0; INPUTS];
        i[0] = height;
        i[1] = up * up;
        i[8] = 0.5;
        i
    }

    #[test]
    fn rules_split_by_height_then_slope() {
        let p = palette(vec![
            rule("Coast", 0, "Sand", "Inland", 0.0, 2.0),
            rule("Inland", 1, "Rock", "Grass", 0.25, 0.81),
        ]);
        let tree = RuleTree::new(&p);
        let coast = RuleTree::node(&p, "coast").unwrap();
        let mut out = [0.0; 3];
        tree.weigh(&[(coast, 1.0)], &inputs(1.0, 1.0), &mut out);
        assert_eq!(out, [0.5, 0.0, 0.5]);
        let mut out = [0.0; 3];
        tree.weigh(&[(coast, 1.0)], &inputs(5.0, 0.5), &mut out);
        assert_eq!(out, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn leftover_weight_goes_to_the_first_node() {
        let p = palette(vec![rule("Fixed", 8, "Sand", "Rock", 0.0, 1.0)]);
        let tree = RuleTree::new(&p);
        let grass = RuleTree::node(&p, "Grass").unwrap();
        let mut out = [0.0; 3];
        tree.weigh(&[(grass, 0.5)], &inputs(0.0, 1.0), &mut out);
        assert_eq!(out, [0.25, 0.25, 0.5]);
    }

    #[test]
    fn cycles_terminate() {
        let p = palette(vec![
            rule("A", 0, "B", "B", 0.0, 1.0),
            rule("B", 0, "A", "A", 0.0, 1.0),
        ]);
        let tree = RuleTree::new(&p);
        let mut out = [0.0; 3];
        tree.weigh(&[(0, 1.0)], &inputs(0.5, 1.0), &mut out);
        assert_eq!(out, [0.0; 3]);
    }

    #[test]
    fn noise_is_deterministic_and_spread() {
        let maps = NoiseMaps::default();
        let again = NoiseMaps::default();
        assert_eq!(maps.maps[0], again.maps[0]);
        assert_ne!(maps.maps[0], maps.maps[1]);
        let mean =
            maps.maps[2].iter().map(|&v| v as f64).sum::<f64>() / (NOISE_SIDE * NOISE_SIDE) as f64;
        assert!((100.0..156.0).contains(&mean), "mean {mean}");
        // Tile (1, 0) continues tile (0, 0) half a map along X.
        assert_eq!(maps.sample(0, 1, 0, 0, 3), maps.sample(0, 0, 0, 240, 3));
        assert_eq!(maps.sample(0, 2, 0, 5, 5), maps.sample(0, 0, 0, 5, 5));
    }

    #[test]
    fn ridge_peaks_mid_range() {
        assert_eq!(ridge(0), 0);
        assert_eq!(ridge(127), 255);
        assert_eq!(ridge(255), 0);
    }

    #[test]
    fn over_alphas_reproduce_weights() {
        let a = over_alphas(&[0.5, 0.25, 0.25]);
        assert_eq!(a, vec![1.0, 1.0 / 3.0, 0.25]);
        // Compositing the alphas in order gives back the weights.
        let mut cover = [1.0f32, 0.0, 0.0];
        for k in 1..3 {
            for c in cover.iter_mut().take(k) {
                *c *= 1.0 - a[k];
            }
            cover[k] = a[k];
        }
        assert!((cover[0] - 0.5).abs() < 1e-6 && (cover[2] - 0.25).abs() < 1e-6);
    }
}
