//! `exa-tool picdiff`: compares two pictures of the same pose (two Exarust
//! runs, or Exarust against a retail shot) and reports how much changed and
//! where, against a noise floor measured from two runs of one build.
//!
//! A pixel has changed when its largest channel difference (red, green or
//! blue, 0 to 255) is above the threshold. The changed pixels are grouped
//! into regions: the picture is cut into square tiles, a tile with at least
//! `min_tile` changed pixels is marked, and touching marked tiles (the
//! eight neighbours) form one region, reported with its pixel bounds.
//!
//! The noise floor is the changed share of a second pair (`--noise c d`,
//! two runs of the same build and scene) at the same threshold. The first
//! pair is at the noise floor when its changed share is no more than
//! `noise_factor` times the floor plus `noise_slack`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// An RGBA picture, eight bits a channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// A `width` by `height` picture filled with one colour.
    pub fn filled(width: u32, height: u32, colour: [u8; 4]) -> Self {
        Self {
            width,
            height,
            rgba: colour.repeat((width * height) as usize),
        }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    fn set(&mut self, x: u32, y: u32, colour: [u8; 4]) {
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&colour);
    }

    /// Reads a PNG of any colour type and depth as RGBA8.
    pub fn read(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).with_context(|| path.display().to_string())?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder
            .read_info()
            .with_context(|| format!("{}: not a PNG", path.display()))?;
        let size = reader
            .output_buffer_size()
            .with_context(|| format!("{}: too large", path.display()))?;
        let mut buf = vec![0; size];
        let info = reader.next_frame(&mut buf)?;
        let (w, h) = (info.width, info.height);
        let pixels = (w * h) as usize;
        let mut rgba = Vec::with_capacity(pixels * 4);
        for y in 0..h as usize {
            let line = &buf[y * info.line_size..];
            for x in 0..w as usize {
                let px = match info.color_type {
                    png::ColorType::Rgba => [
                        line[x * 4],
                        line[x * 4 + 1],
                        line[x * 4 + 2],
                        line[x * 4 + 3],
                    ],
                    png::ColorType::Rgb => [line[x * 3], line[x * 3 + 1], line[x * 3 + 2], 255],
                    png::ColorType::GrayscaleAlpha => {
                        let g = line[x * 2];
                        [g, g, g, line[x * 2 + 1]]
                    }
                    png::ColorType::Grayscale => {
                        let g = line[x];
                        [g, g, g, 255]
                    }
                    png::ColorType::Indexed => bail!("{}: palette not expanded", path.display()),
                };
                rgba.extend_from_slice(&px);
            }
        }
        Ok(Self {
            width: w,
            height: h,
            rgba,
        })
    }

    /// Bilinear resample to `width` by `height` (a retail shot taken at
    /// another size).
    pub fn resized(&self, width: u32, height: u32) -> Self {
        let mut out = Picture::filled(width, height, [0; 4]);
        let sx = self.width as f32 / width as f32;
        let sy = self.height as f32 / height as f32;
        for y in 0..height {
            let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (self.height - 1) as f32);
            let (y0, ty) = (fy.floor() as u32, fy.fract());
            let y1 = (y0 + 1).min(self.height - 1);
            for x in 0..width {
                let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (self.width - 1) as f32);
                let (x0, tx) = (fx.floor() as u32, fx.fract());
                let x1 = (x0 + 1).min(self.width - 1);
                let (a, b, c, d) = (
                    self.at(x0, y0),
                    self.at(x1, y0),
                    self.at(x0, y1),
                    self.at(x1, y1),
                );
                let mut px = [0u8; 4];
                for k in 0..4 {
                    let top = a[k] as f32 * (1.0 - tx) + b[k] as f32 * tx;
                    let bottom = c[k] as f32 * (1.0 - tx) + d[k] as f32 * tx;
                    px[k] = (top * (1.0 - ty) + bottom * ty).round() as u8;
                }
                out.set(x, y, px);
            }
        }
        out
    }

    /// Writes the picture as an RGBA PNG.
    pub fn write(&self, path: &Path) -> Result<()> {
        crate::write_png(path, self.width, self.height, &self.rgba)
    }
}

/// A rectangle in pixels: `x`, `y`, width, height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// `x,y,w,h`.
    pub fn parse(word: &str) -> Option<Self> {
        let v: Vec<u32> = word
            .split(',')
            .map(|p| p.trim().parse().ok())
            .collect::<Option<_>>()?;
        match v.as_slice() {
            [x, y, w, h] => Some(Rect {
                x: *x,
                y: *y,
                w: *w,
                h: *h,
            }),
            _ => None,
        }
    }

    fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// How two pictures are compared.
#[derive(Debug, Clone)]
pub struct Settings {
    /// A pixel changed when its largest channel difference is above this.
    pub threshold: u8,
    /// The tile side the regions are built from, in pixels.
    pub tile: u32,
    /// Changed pixels a tile needs to be part of a region.
    pub min_tile: u32,
    /// Rectangles left out of the comparison (a frame counter, a HUD).
    pub ignore: Vec<Rect>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            threshold: 8,
            tile: 16,
            min_tile: 4,
            ignore: Vec::new(),
        }
    }
}

/// A connected group of marked tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub bounds: Rect,
    /// Changed pixels inside the region's tiles.
    pub changed: u32,
}

/// What a comparison found.
#[derive(Debug, Clone)]
pub struct Diff {
    pub width: u32,
    pub height: u32,
    /// Pixels compared (the picture less the ignored rectangles).
    pub compared: u64,
    /// Pixels above the threshold.
    pub changed: u64,
    /// The largest channel difference anywhere.
    pub max: u8,
    /// Mean of the per-pixel largest channel difference.
    pub mean: f64,
    /// The 99th and 99.9th percentiles of the per-pixel difference.
    pub p99: u8,
    pub p999: u8,
    /// Peak signal-to-noise ratio over RGB in dB (infinite when equal).
    pub psnr: f64,
    /// Regions, the most changed first.
    pub regions: Vec<Region>,
    /// The per-pixel largest channel difference (0 for ignored pixels).
    pub per_pixel: Vec<u8>,
}

impl Diff {
    /// The changed share of the compared pixels.
    pub fn share(&self) -> f64 {
        if self.compared == 0 {
            0.0
        } else {
            self.changed as f64 / self.compared as f64
        }
    }
}

/// Compares two pictures of one size.
pub fn compare(a: &Picture, b: &Picture, settings: &Settings) -> Result<Diff> {
    if (a.width, a.height) != (b.width, b.height) {
        bail!(
            "sizes differ: {}x{} and {}x{} (pass --fit to resample the second)",
            a.width,
            a.height,
            b.width,
            b.height
        );
    }
    let (w, h) = (a.width, a.height);
    let mut per_pixel = vec![0u8; (w * h) as usize];
    let mut histogram = [0u64; 256];
    let (mut compared, mut changed, mut max, mut sum, mut squares) = (0u64, 0u64, 0u8, 0u64, 0f64);
    let tiles_x = w.div_ceil(settings.tile);
    let tiles_y = h.div_ceil(settings.tile);
    let mut tile_counts = vec![0u32; (tiles_x * tiles_y) as usize];
    for y in 0..h {
        for x in 0..w {
            if settings.ignore.iter().any(|r| r.contains(x, y)) {
                continue;
            }
            let (pa, pb) = (a.at(x, y), b.at(x, y));
            let mut d = 0u8;
            for k in 0..3 {
                let c = pa[k].abs_diff(pb[k]);
                d = d.max(c);
                squares += f64::from(c) * f64::from(c);
            }
            compared += 1;
            histogram[d as usize] += 1;
            sum += u64::from(d);
            max = max.max(d);
            per_pixel[(y * w + x) as usize] = d;
            if d > settings.threshold {
                changed += 1;
                tile_counts[((y / settings.tile) * tiles_x + x / settings.tile) as usize] += 1;
            }
        }
    }
    let percentile = |q: f64| -> u8 {
        let target = (compared as f64 * q).ceil() as u64;
        let mut seen = 0;
        for (value, count) in histogram.iter().enumerate() {
            seen += count;
            if seen >= target.max(1) {
                return value as u8;
            }
        }
        255
    };
    let mse = if compared == 0 {
        0.0
    } else {
        squares / (compared as f64 * 3.0)
    };
    let psnr = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0f64 * 255.0 / mse).log10()
    };
    let regions = regions(&tile_counts, tiles_x, tiles_y, w, h, settings);
    Ok(Diff {
        width: w,
        height: h,
        compared,
        changed,
        max,
        mean: if compared == 0 {
            0.0
        } else {
            sum as f64 / compared as f64
        },
        p99: percentile(0.99),
        p999: percentile(0.999),
        psnr,
        regions,
        per_pixel,
    })
}

/// Groups the marked tiles into regions (eight-neighbour flood fill).
fn regions(
    counts: &[u32],
    tiles_x: u32,
    tiles_y: u32,
    w: u32,
    h: u32,
    settings: &Settings,
) -> Vec<Region> {
    let marked = |i: usize| counts[i] >= settings.min_tile.max(1);
    let mut seen = vec![false; counts.len()];
    let mut out = Vec::new();
    for start in 0..counts.len() {
        if seen[start] || !marked(start) {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![start];
        let (mut x0, mut y0, mut x1, mut y1, mut changed) = (u32::MAX, u32::MAX, 0, 0, 0);
        while let Some(i) = stack.pop() {
            let (tx, ty) = (i as u32 % tiles_x, i as u32 / tiles_x);
            x0 = x0.min(tx);
            y0 = y0.min(ty);
            x1 = x1.max(tx);
            y1 = y1.max(ty);
            changed += counts[i];
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (tx as i32 + dx, ty as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= tiles_x as i32 || ny >= tiles_y as i32 {
                        continue;
                    }
                    let n = (ny as u32 * tiles_x + nx as u32) as usize;
                    if !seen[n] && marked(n) {
                        seen[n] = true;
                        stack.push(n);
                    }
                }
            }
        }
        let t = settings.tile;
        let (px, py) = (x0 * t, y0 * t);
        out.push(Region {
            bounds: Rect {
                x: px,
                y: py,
                w: ((x1 + 1) * t).min(w) - px,
                h: ((y1 + 1) * t).min(h) - py,
            },
            changed,
        });
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.changed));
    out
}

const BOX: [u8; 4] = [255, 220, 0, 255];

fn outline(picture: &mut Picture, r: Rect, colour: [u8; 4]) {
    if r.w == 0 || r.h == 0 {
        return;
    }
    let (x1, y1) = (r.x + r.w - 1, r.y + r.h - 1);
    for x in r.x..=x1 {
        picture.set(x, r.y, colour);
        picture.set(x, y1, colour);
    }
    for y in r.y..=y1 {
        picture.set(r.x, y, colour);
        picture.set(x1, y, colour);
    }
}

/// The diff picture: the first picture dimmed to grey, each changed pixel
/// red by how much it changed, the regions boxed in yellow.
pub fn diff_picture(a: &Picture, diff: &Diff, settings: &Settings, boxes: usize) -> Picture {
    let mut out = a.clone();
    for y in 0..a.height {
        for x in 0..a.width {
            let [r, g, b, _] = a.at(x, y);
            let grey = ((u32::from(r) * 3 + u32::from(g) * 6 + u32::from(b)) / 40) as u8;
            let d = diff.per_pixel[(y * a.width + x) as usize];
            let px = if d > settings.threshold {
                let red = (128 + u32::from(d) / 2).min(255) as u8;
                [red, grey / 2, grey / 2, 255]
            } else {
                [grey, grey, grey, 255]
            };
            out.set(x, y, px);
        }
    }
    for region in diff.regions.iter().take(boxes) {
        outline(&mut out, region.bounds, BOX);
    }
    out
}

/// The side-by-side picture: the first picture, a gap, the second, the
/// regions boxed on both.
pub fn side_by_side(a: &Picture, b: &Picture, diff: &Diff, boxes: usize) -> Picture {
    const GAP: u32 = 8;
    let mut out = Picture::filled(a.width * 2 + GAP, a.height, [0, 0, 0, 255]);
    for y in 0..a.height {
        for x in 0..a.width {
            out.set(x, y, a.at(x, y));
            out.set(x + a.width + GAP, y, b.at(x, y));
        }
    }
    for region in diff.regions.iter().take(boxes) {
        outline(&mut out, region.bounds, BOX);
        let mut right = region.bounds;
        right.x += a.width + GAP;
        outline(&mut out, right, BOX);
    }
    out
}

/// The verdict against a noise floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor {
    /// The noise pair's changed share.
    pub share: f64,
    /// The largest share still called noise.
    pub limit: f64,
    pub at_floor: bool,
}

/// The first pair against the noise pair's changed share: at the floor
/// when no more than `factor` times it plus `slack`.
pub fn against_floor(diff: &Diff, noise: &Diff, factor: f64, slack: f64) -> Floor {
    let limit = noise.share() * factor + slack;
    Floor {
        share: noise.share(),
        limit,
        at_floor: diff.share() <= limit,
    }
}

/// The `picdiff` command's arguments.
#[derive(Debug, Clone)]
pub struct Args {
    pub a: PathBuf,
    pub b: PathBuf,
    pub noise: Vec<PathBuf>,
    pub settings: Settings,
    pub fit: bool,
    pub out: Option<PathBuf>,
    pub json: Option<PathBuf>,
    pub regions: usize,
    pub noise_factor: f64,
    pub noise_slack: f64,
}

fn read_pair(a: &Path, b: &Path, fit: bool) -> Result<(Picture, Picture)> {
    let pa = Picture::read(a)?;
    let mut pb = Picture::read(b)?;
    if fit && (pa.width, pa.height) != (pb.width, pb.height) {
        println!(
            "fit: {} resampled from {}x{} to {}x{}",
            b.display(),
            pb.width,
            pb.height,
            pa.width,
            pa.height
        );
        pb = pb.resized(pa.width, pa.height);
    }
    Ok((pa, pb))
}

fn summary(label: &str, diff: &Diff, threshold: u8) {
    println!(
        "{label}: {}x{}, {} of {} pixels changed above {threshold} ({:.4}%), max {}, mean {:.3}, p99 {}, p99.9 {}, PSNR {:.2} dB, {} regions",
        diff.width,
        diff.height,
        diff.changed,
        diff.compared,
        diff.share() * 100.0,
        diff.max,
        diff.mean,
        diff.p99,
        diff.p999,
        diff.psnr,
        diff.regions.len()
    );
}

/// Runs `picdiff`: prints the summary, the regions and the noise verdict,
/// and writes the pictures and the JSON report asked for. Returns whether
/// the pair is at the noise floor (true without a noise pair).
pub fn run(args: &Args) -> Result<bool> {
    let (a, b) = read_pair(&args.a, &args.b, args.fit)?;
    let diff = compare(&a, &b, &args.settings)?;
    summary("pair", &diff, args.settings.threshold);
    for (i, region) in diff.regions.iter().take(args.regions).enumerate() {
        let r = region.bounds;
        println!(
            "  region {}: x {} y {} w {} h {}, {} changed pixels",
            i + 1,
            r.x,
            r.y,
            r.w,
            r.h,
            region.changed
        );
    }
    if diff.regions.len() > args.regions {
        println!("  ({} more regions)", diff.regions.len() - args.regions);
    }
    let floor = match args.noise.as_slice() {
        [] => None,
        [c, d] => {
            let (c, d) = read_pair(c, d, args.fit)?;
            let noise = compare(&c, &d, &args.settings)?;
            summary("noise", &noise, args.settings.threshold);
            let floor = against_floor(&diff, &noise, args.noise_factor, args.noise_slack);
            println!(
                "noise floor {:.4}% (limit {:.4}%): the pair is {}",
                floor.share * 100.0,
                floor.limit * 100.0,
                if floor.at_floor {
                    "AT the noise floor"
                } else {
                    "ABOVE the noise floor"
                }
            );
            Some(floor)
        }
        _ => bail!("--noise takes two pictures"),
    };
    if let Some(out) = &args.out {
        let stem = out.to_string_lossy();
        let side = PathBuf::from(format!("{stem}.side.png"));
        let pic = PathBuf::from(format!("{stem}.diff.png"));
        if let Some(dir) = side.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        side_by_side(&a, &b, &diff, args.regions).write(&side)?;
        diff_picture(&a, &diff, &args.settings, args.regions).write(&pic)?;
        println!("wrote {} and {}", side.display(), pic.display());
    }
    if let Some(path) = &args.json {
        let regions: Vec<_> = diff
            .regions
            .iter()
            .map(|r| {
                serde_json::json!({
                    "x": r.bounds.x, "y": r.bounds.y, "w": r.bounds.w, "h": r.bounds.h,
                    "changed": r.changed,
                })
            })
            .collect();
        let report = serde_json::json!({
            "a": args.a, "b": args.b,
            "width": diff.width, "height": diff.height,
            "threshold": args.settings.threshold,
            "compared": diff.compared, "changed": diff.changed, "share": diff.share(),
            "max": diff.max, "mean": diff.mean, "p99": diff.p99, "p999": diff.p999,
            "psnr": if diff.psnr.is_finite() { serde_json::json!(diff.psnr) } else { serde_json::Value::Null },
            "regions": regions,
            "noise": floor.map(|f| serde_json::json!({
                "share": f.share, "limit": f.limit, "at_floor": f.at_floor,
            })),
        });
        fs::write(path, serde_json::to_string_pretty(&report)?)?;
        println!("wrote {}", path.display());
    }
    Ok(floor.is_none_or(|f| f.at_floor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_block(base: &Picture, r: Rect, colour: [u8; 4]) -> Picture {
        let mut p = base.clone();
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                p.set(x, y, colour);
            }
        }
        p
    }

    #[test]
    fn equal_pictures_change_nothing() {
        let a = Picture::filled(64, 48, [40, 50, 60, 255]);
        let diff = compare(&a, &a, &Settings::default()).unwrap();
        assert_eq!(diff.changed, 0);
        assert_eq!(diff.compared, 64 * 48);
        assert!(diff.regions.is_empty());
        assert!(diff.psnr.is_infinite());
    }

    #[test]
    fn a_changed_block_is_one_region_with_its_bounds() {
        let a = Picture::filled(128, 64, [40, 50, 60, 255]);
        let block = Rect {
            x: 32,
            y: 16,
            w: 32,
            h: 16,
        };
        let b = with_block(&a, block, [200, 50, 60, 255]);
        let diff = compare(&a, &b, &Settings::default()).unwrap();
        assert_eq!(diff.changed, 32 * 16);
        assert_eq!(diff.max, 160);
        assert_eq!(diff.regions.len(), 1);
        assert_eq!(diff.regions[0].bounds, block);
        // A second, separate block is a second region; the larger first.
        let far = Rect {
            x: 100,
            y: 48,
            w: 8,
            h: 8,
        };
        let c = with_block(&b, far, [0, 0, 0, 255]);
        let diff = compare(&a, &c, &Settings::default()).unwrap();
        assert_eq!(diff.regions.len(), 2);
        assert_eq!(diff.regions[0].bounds, block);
        assert_eq!(diff.regions[1].changed, 64);
    }

    #[test]
    fn differences_at_or_under_the_threshold_and_ignored_pixels_do_not_count() {
        let a = Picture::filled(32, 32, [100, 100, 100, 255]);
        let b = Picture::filled(32, 32, [108, 100, 100, 255]);
        let diff = compare(&a, &b, &Settings::default()).unwrap();
        assert_eq!(diff.changed, 0);
        assert_eq!(diff.max, 8);
        let c = with_block(
            &a,
            Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 4,
            },
            [0, 0, 0, 255],
        );
        let settings = Settings {
            ignore: vec![Rect::parse("0,0,4,4").unwrap()],
            ..Settings::default()
        };
        let diff = compare(&a, &c, &settings).unwrap();
        assert_eq!(diff.changed, 0);
        assert_eq!(diff.compared, 32 * 32 - 16);
    }

    #[test]
    fn the_noise_floor_judges_the_pair() {
        let a = Picture::filled(100, 100, [10, 10, 10, 255]);
        let speck = |n: u32| {
            with_block(
                &a,
                Rect {
                    x: 0,
                    y: 0,
                    w: n,
                    h: 1,
                },
                [250, 10, 10, 255],
            )
        };
        let noise = compare(&a, &speck(10), &Settings::default()).unwrap();
        let quiet = compare(&a, &speck(12), &Settings::default()).unwrap();
        let loud = compare(&a, &speck(100), &Settings::default()).unwrap();
        assert!(against_floor(&quiet, &noise, 1.5, 0.0).at_floor);
        assert!(!against_floor(&loud, &noise, 1.5, 0.0).at_floor);
    }

    #[test]
    fn the_pictures_are_written_and_read_back() {
        let dir = std::env::temp_dir().join(format!("picdiff-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let a = Picture::filled(20, 10, [1, 2, 3, 255]);
        let b = with_block(
            &a,
            Rect {
                x: 0,
                y: 0,
                w: 16,
                h: 10,
            },
            [200, 2, 3, 255],
        );
        let (pa, pb) = (dir.join("a.png"), dir.join("b.png"));
        a.write(&pa).unwrap();
        b.write(&pb).unwrap();
        assert_eq!(Picture::read(&pa).unwrap(), a);
        let args = Args {
            a: pa.clone(),
            b: pb,
            noise: vec![pa.clone(), pa],
            settings: Settings::default(),
            fit: false,
            out: Some(dir.join("out")),
            json: Some(dir.join("out.json")),
            regions: 10,
            noise_factor: 1.5,
            noise_slack: 0.0,
        };
        assert!(!run(&args).unwrap());
        let side = Picture::read(&dir.join("out.side.png")).unwrap();
        assert_eq!((side.width, side.height), (48, 10));
        assert!(dir.join("out.diff.png").is_file());
        assert!(dir.join("out.json").is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_resample_keeps_a_flat_picture_flat() {
        let a = Picture::filled(40, 20, [9, 99, 199, 255]);
        let r = a.resized(25, 13);
        assert_eq!((r.width, r.height), (25, 13));
        assert_eq!(r, Picture::filled(25, 13, [9, 99, 199, 255]));
    }
}
