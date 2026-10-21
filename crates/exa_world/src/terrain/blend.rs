//! Terrain blend rules: painted layers resolved, per vertex, into weights of
//! the palette's materials.
//!
//! Rules and materials form one node list (rules first). A rule reads one
//! per-vertex input selected by its kind and sends its weight to its first
//! child below `lo`, to its second above `hi`, and splits it linearly in
//! between. Weight the painted layers leave over goes to node 0. Evidence:
//! `docs/formats/TERRAIN.md`.

use exa_formats::terrain::{BlendRule, Palette};
use exa_render::image_noise::{ridge_bytes, value_noise_bytes};

/// Side of the repeating noise maps, in terrain vertices (96 m).
pub const NOISE_SIDE: usize = 480;
/// Noise maps are generated from these seeds (inputs 3, 4 and 7).
const NOISE_SEEDS: [u32; 3] = [137, 476, 7359];
const NOISE_OCTAVES: u32 = 4;
/// Leftover weight below this is ignored.
const LEFTOVER_EPSILON: f32 = 0.001;
/// Deeper rule chains are treated as cycles.
const MAX_DEPTH: usize = 32;
/// The most painted brushes a tile vertex evaluates; the original stops at
/// ten and reports "Maximum brushes exceeded."
pub const MAX_BRUSHES: usize = 10;
/// One material channel's side: the original allocates 256 × 256 bytes per
/// channel.
pub const CHANNEL_SIDE: usize = 256;

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
    /// The three maps as the terrain renderer builds them: value noise of
    /// four octaves per seed, the first folded into a ridge.
    fn default() -> Self {
        let mut maps =
            NOISE_SEEDS.map(|seed| value_noise_bytes(NOISE_SIDE, NOISE_SIDE, NOISE_OCTAVES, seed));
        ridge_bytes(&mut maps[0]);
        Self { maps }
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
        let a = self.sample(0, tile_x, tile_y, i, j) as f32 / 255.0;
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

/// One entry of the original's name-sorted brush table: a 16-byte
/// lower-case name, the node it names and its tag (0 for a rule, the
/// material index for a material).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrushEntry {
    pub name: [u8; 16],
    pub node: usize,
    pub tag: i32,
}

/// The original's brush table: rules inserted first, then materials, each
/// by binary search into a list sorted by the lower-case name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BrushTable {
    entries: Vec<BrushEntry>,
}

impl BrushTable {
    /// Builds the table from a palette: its rules first (tag 0, the rule's
    /// index as the node), then its materials (tag the material index, the
    /// node after the rules). The original first inserts every name into
    /// its sorted list and then keeps the mapped record.
    pub fn of(palette: &Palette) -> Self {
        let mut table = Self::default();
        for (at, rule) in palette.rules.iter().enumerate() {
            table.insert(&rule.name, at, 0);
        }
        for (at, material) in palette.materials.iter().enumerate() {
            table.insert(&material.name, palette.rules.len() + at, at as i32);
        }
        table
    }

    /// Inserts one entry before the first name not below it, shifting the
    /// tail (the original's binary-search insertion).
    pub fn insert(&mut self, name: &str, node: usize, tag: i32) {
        let mut key = [0u8; 16];
        for (out, byte) in key.iter_mut().zip(name.as_bytes()) {
            *out = byte.to_ascii_lowercase();
        }
        let entry = BrushEntry {
            name: key,
            node,
            tag,
        };
        let at = self.entries.partition_point(|other| other.name < key);
        self.entries.insert(at, entry);
    }

    /// The entry named `name`, compared as the original's 16 lower-case
    /// bytes; the last of duplicate names wins, as its later insertion
    /// shadows the earlier one.
    pub fn get(&self, name: &str) -> Option<&BrushEntry> {
        let mut key = [0u8; 16];
        for (out, byte) in key.iter_mut().zip(name.as_bytes()) {
            *out = byte.to_ascii_lowercase();
        }
        self.entries.iter().rev().find(|entry| entry.name == key)
    }

    /// The entries in sorted order.
    pub fn entries(&self) -> &[BrushEntry] {
        &self.entries
    }

    /// How many entries the table holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A tile's painted brushes resolved against the table, with the base
/// tree's node appended. The original allows at most [`MAX_BRUSHES`] and
/// reports "Maximum brushes exceeded." for the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brushes {
    /// Painted nodes, in the tile's order.
    pub nodes: Vec<usize>,
    /// The base rule tree's node (`0`, the first rule).
    pub base: usize,
    /// How many painted layers were past the limit.
    pub exceeded: usize,
}

/// Resolves a tile's layer names into at most ten brushes plus the base
/// tree. Unknown names are left out, as the original's failed lookup.
pub fn brushes<'a>(table: &BrushTable, layers: impl IntoIterator<Item = &'a str>) -> Brushes {
    let mut nodes = Vec::new();
    let mut exceeded = 0;
    for name in layers {
        let Some(entry) = table.get(name) else {
            continue;
        };
        if nodes.len() >= MAX_BRUSHES {
            exceeded += 1;
            continue;
        }
        nodes.push(entry.node);
    }
    Brushes {
        nodes,
        base: 0,
        exceeded,
    }
}

/// Scales the rule height bounds by `scale` and adds `offset`, in the
/// original's truncating integer arithmetic. Only rules (tag 0) are
/// touched; materials keep their bounds.
pub fn scale_rule_heights(rules: &mut [BlendRule], scale: f32, offset: f32) {
    for rule in rules {
        rule.lo = ((rule.lo * scale + offset) as i32) as f32;
        rule.hi = ((rule.hi * scale + offset) as i32) as f32;
    }
}

/// Adds one vertex's weight to a material channel's byte, rounding the
/// weight to 0..=255 and saturating: the original's accumulator.
pub fn accumulate(current: u8, weight: f32) -> u8 {
    let add = (weight * 255.0).round_ties_even();
    let add = if add < 0.0 {
        0
    } else if add > 255.0 {
        255
    } else {
        add as u8
    };
    current.saturating_add(add)
}

/// Packs used materials into interleaved mask planes, four per plane: the
/// first plane of a group holds up to four materials' bytes per texel,
/// laid out material by material at `texel·group + lane`; the next group
/// starts a plane later. `mask(material, texel)` gives each byte. This is
/// the original's four-per-RGBA-mask packing, with a short last group
/// packed tighter.
pub fn pack_material_masks(
    used: usize,
    texels: usize,
    mask: impl Fn(usize, usize) -> u8,
) -> Vec<u8> {
    let mut out = vec![0u8; used.saturating_mul(texels)];
    let mut start = 0;
    while start < used {
        let group = (used - start).min(4);
        let base = start * texels;
        for texel in 0..texels {
            for lane in 0..group {
                out[base + texel * group + lane] = mask(start + lane, texel);
            }
        }
        start += group;
    }
    out
}

/// A tile's material weights from the painted brushes and the leftover,
/// before packing: one channel of 256 × 256 bytes per material, each byte
/// accumulated the original's way. `weigh` is the per-vertex evaluator.
pub fn channel_bytes(
    materials: usize,
    vertices: usize,
    mut weigh: impl FnMut(usize) -> Vec<f32>,
) -> Vec<u8> {
    let mut out = vec![0u8; materials.saturating_mul(vertices)];
    for vertex in 0..vertices {
        for (material, weight) in weigh(vertex).into_iter().enumerate() {
            if material >= materials {
                break;
            }
            let at = material * vertices + vertex;
            out[at] = accumulate(out[at], weight);
        }
    }
    out
}

/// The blur kernel the terrain texture pass builds: half the Gaussian on
/// the negative side (zero included), mirrored to the positive side, then
/// normalised to a sum of one. `radius` is the half-width; the kernel has
/// `2·radius + 1` entries. The exponential is the library's double one, as
/// the original's, with the sigma a third of the radius.
pub fn gaussian_kernel(radius: i32) -> Vec<f32> {
    let count = (radius * 2 + 1).max(1) as usize;
    let mut kernel = vec![0.0f32; count];
    let r = radius as f32;
    let sigma = r * 0.333_333_34;
    let denominator = sigma * sigma * 2.0;
    let mut i = -radius;
    loop {
        let square = (i * i) as f32;
        kernel[(i + radius) as usize] = f64::exp(f64::from(-square / denominator)) as f32;
        if i >= 0 {
            break;
        }
        i += 1;
    }
    for i in 1..=radius {
        kernel[(i + radius) as usize] = kernel[(radius - i) as usize];
    }
    let sum: f32 = kernel.iter().sum();
    if sum != 0.0 {
        for value in &mut kernel {
            *value /= sum;
        }
    }
    kernel
}

/// A kernel widened to four floats an entry (the original's 16-byte
/// entries), each of the four lanes holding the same weight.
pub fn expand_kernel(kernel: &[f32]) -> Vec<[f32; 4]> {
    kernel.iter().map(|&v| [v; 4]).collect()
}

/// Adds a material's mask into the running weight and rescales the mask to
/// its share of it: `w += byte`, `byte = min(255, round(byte·255 / w))`.
/// A zero weight leaves the mask alone (the division would be infinite).
pub fn merge_into(weights: &mut [f32], mask: &mut [u8]) {
    for (weight, byte) in weights.iter_mut().zip(mask.iter_mut()) {
        *weight += f32::from(*byte);
        if *weight != 0.0 {
            let scaled = (f32::from(*byte) * 255.0 / *weight).round_ties_even();
            *byte = scaled.min(255.0) as u8;
        }
    }
}

/// The original's used-material merge: the weights start as the first
/// material's bytes, then every material but the last merges into them.
pub fn merge_material_masks(first: &[u8], rest: &mut [Vec<u8>], texels: usize) -> Vec<f32> {
    let mut weights = vec![0.0f32; texels];
    for (weight, byte) in weights.iter_mut().zip(first.iter().copied()) {
        *weight = f32::from(byte);
    }
    let last = rest.len().saturating_sub(1);
    for mask in rest.iter_mut().take(last) {
        merge_into(&mut weights, mask);
    }
    weights
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
            environment: 0.0,
            wetness: 0.0,
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
    fn the_first_map_is_ridged_value_noise() {
        let maps = NoiseMaps::default();
        // The original's seed-137 map, folded: its first texels and its sum
        // (worked from the rules by a separate model).
        assert_eq!(maps.maps[0][..8], [137, 128, 117, 131, 137, 112, 134, 143]);
        let sum: u64 = maps.maps[0].iter().map(|&v| u64::from(v)).sum();
        assert_eq!(sum, 40_696_892);
        assert_eq!(maps.maps[1][..4], [156, 161, 159, 167]);
        let inputs = maps.inputs(0.0, 1.0, 0, 0, 0, 0);
        assert_eq!(inputs[3], 137.0 / 255.0);
        assert_eq!(inputs[6], inputs[3]);
        assert_eq!(inputs[4], 156.0 / 255.0);
    }

    #[test]
    fn a_brush_table_is_sorted_by_lower_case_name() {
        let p = palette(vec![
            rule("Coast", 0, "Sand", "Rock", 0.0, 2.0),
            rule("Inland", 1, "Rock", "Grass", 0.25, 0.81),
        ]);
        let table = BrushTable::of(&p);
        let names: Vec<String> = table
            .entries()
            .iter()
            .map(|e| {
                String::from_utf8_lossy(&e.name)
                    .trim_end_matches('\0')
                    .to_string()
            })
            .collect();
        assert_eq!(names, ["coast", "grass", "inland", "rock", "sand"]);
        assert_eq!(table.get("COAST").map(|e| e.node), Some(0));
        assert_eq!(table.get("grass").map(|e| e.tag), Some(2));
        assert_eq!(table.get("missing"), None);
        assert_eq!(table.len(), 5);
        let brushes = brushes(&table, ["coast", "missing", "INLAND"]);
        assert_eq!(brushes.nodes, [0, 1]);
        assert_eq!(brushes.base, 0);
        assert_eq!(brushes.exceeded, 0);
    }

    #[test]
    fn only_ten_brushes_fit() {
        let p = palette(
            (0..12)
                .map(|i| rule(&format!("r{i}"), 0, "Sand", "Rock", 0.0, 1.0))
                .collect(),
        );
        let table = BrushTable::of(&p);
        let b = brushes(
            &table,
            (0..12)
                .map(|i| format!("r{i}"))
                .collect::<Vec<_>>()
                .iter()
                .map(String::as_str),
        );
        assert_eq!(b.nodes.len(), MAX_BRUSHES);
        assert_eq!(b.exceeded, 2);
    }

    #[test]
    fn rule_heights_scale_with_truncation() {
        let mut rules = vec![
            rule("a", 0, "Sand", "Rock", 0.25, 0.75),
            rule("b", 1, "Rock", "Grass", -0.5, 1.5),
        ];
        scale_rule_heights(&mut rules, 100.0, 5.0);
        assert_eq!((rules[0].lo, rules[0].hi), (30.0, 80.0));
        assert_eq!((rules[1].lo, rules[1].hi), (-45.0, 155.0));
    }

    #[test]
    fn weights_accumulate_into_channels_and_pack_four_a_plane() {
        assert_eq!(accumulate(0, 0.5), 128);
        assert_eq!(accumulate(200, 0.5), 255);
        assert_eq!(accumulate(10, 0.0), 10);
        let channels = channel_bytes(2, 3, |v| vec![v as f32 / 3.0, 1.0 - v as f32 / 3.0]);
        // Material-major channels: each material's whole channel first.
        assert_eq!(channels[0], 0);
        assert_eq!(channels[1], 85);
        assert_eq!(channels[2], 170);
        assert_eq!(channels[3], 255);
        assert_eq!(channels[4], 170);
        assert_eq!(channels[5], 85);
        let packed = pack_material_masks(5, 2, |m, v| (m * 10 + v) as u8);
        // Group 0 holds materials 0..4 interleaved at texel·4+lane; group 1
        // starts at material 4's plane and holds one byte a texel.
        assert_eq!(packed[..8], [0, 10, 20, 30, 1, 11, 21, 31]);
        assert_eq!(packed[4 * 2], 40);
        assert_eq!(packed[4 * 2 + 1], 41);
    }

    #[test]
    fn a_gaussian_kernel_is_mirrored_and_normalised() {
        let k = gaussian_kernel(1);
        assert_eq!(k.len(), 3);
        assert!(k[0] == k[2]);
        assert!((k.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!((k[1] - 0.978_24).abs() < 1e-4, "{k:?}");
        assert!((k[0] - 0.010_88).abs() < 1e-4, "{k:?}");
        let wide = gaussian_kernel(2);
        assert_eq!(wide.len(), 5);
        assert!(wide[0] == wide[4] && wide[1] == wide[3]);
        assert!((wide.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(expand_kernel(&[0.25, 0.5, 0.25])[1], [0.5; 4]);
    }

    #[test]
    fn a_material_mask_merges_into_the_running_weight() {
        let mut weights = vec![100.0, 0.0];
        let mut mask = vec![100u8, 255];
        merge_into(&mut weights, &mut mask);
        assert_eq!(weights, [200.0, 255.0]);
        assert_eq!(mask, [128, 255]);
        let first = [100u8, 50];
        let mut rest = vec![vec![100u8, 50], vec![200u8, 0]];
        let weights = merge_material_masks(&first, &mut rest, 2);
        // The first material and the middle one merge; the last is skipped.
        assert_eq!(weights, [200.0, 100.0]);
        assert_eq!(rest[0], [128, 128]);
        assert_eq!(rest[1], [200, 0]);
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
