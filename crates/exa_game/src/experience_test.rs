use super::*;
use exa_formats::weapon_stats::at;

fn skills_with(points: [u16; SKILLS]) -> [Skill; SKILLS] {
    points.map(|points| Skill {
        points,
        techniques: [0; TECHNIQUE_SLOTS],
    })
}

/// A learner training skill 0 toward level 1.
fn trainee(experience: u32, points: u16) -> Learner {
    let mut skills = skills_with([points, 0, 0, 0, 0, 0]);
    skills[0].techniques[0] = 1;
    Learner {
        experience,
        skills,
        ..Learner::default()
    }
}

/// 12000 global experience is worth (5000·4 + 5000·5 + 2000·6) / 4 =
/// 14250, and that worth costs 12000 back. 5001 is worth 20005 / 4 = 5001.
#[test]
fn global_experience_is_worth_more_each_step() {
    assert_eq!(global_xp_worth(12_000), 14_250);
    assert_eq!(global_xp_for_worth(14_250), 12_000);
    assert_eq!(global_xp_worth(5_001), 5_001);
    assert_eq!(global_xp_worth(0), 0);
    assert_eq!(global_xp_from_setting(&[0x10, 0x27, 0, 0]), Some(10_000));
    assert_eq!(global_xp_from_setting(&[0x05]), Some(5));
    assert_eq!(global_xp_from_setting(&[]), None);
}

/// The trained skill is the first under level 5 with a technique chosen
/// at its level. A lump of 1000 at rank 1 adds 1000; from 4500 it snaps to
/// 5000; at rank 6 it adds 166; a rank above the lump adds nothing.
#[test]
fn a_lump_goes_to_the_trained_skill() {
    let mut skills = skills_with([0; SKILLS]);
    assert_eq!(training_skill(&skills), None);
    skills[3].techniques[0] = 2;
    assert_eq!(training_skill(&skills), Some((3, 1)));
    assert!(!add_skill_lump(&mut skills, 1000));
    assert_eq!(skills[3].points, 1000);
    skills[3].points = 4500;
    assert!(add_skill_lump(&mut skills, 1000));
    assert_eq!(skills[3].points, 5000);
    let mut ranked = skills_with([0, 5000, 5000, 5000, 5000, 5000]);
    ranked[0].techniques[0] = 1;
    assert!(!add_skill_lump(&mut ranked, 1000));
    assert_eq!(ranked[0].points, 166);
    assert!(!add_skill_lump(&mut ranked, 5));
    assert_eq!(ranked[0].points, 166);
    // A skill at level 5 trains nothing.
    let mut done = skills_with([25_000, 0, 0, 0, 0, 0]);
    done[0].techniques = [1, 2, 3, 4, 5, 6];
    assert_eq!(training_skill(&done), None);
}

/// Total level 0 takes Armour's technique 1; level 1 its technique 2; level
/// 14 Close Combat's technique 8; level 15 nothing.
#[test]
fn a_companion_picks_its_next_technique() {
    let mut skills = skills_with([0; SKILLS]);
    auto_technique(&mut skills);
    assert_eq!(skills[2].techniques[0], 1);
    let mut one = skills_with([5000, 0, 0, 0, 0, 0]);
    auto_technique(&mut one);
    assert_eq!(one[2].techniques[0], 2);
    let mut fourteen = skills_with([10_000, 20_000, 20_000, 20_000, 0, 0]);
    auto_technique(&mut fourteen);
    assert_eq!(fourteen[0].techniques[2], 8);
    let mut fifteen = skills_with([15_000, 20_000, 20_000, 20_000, 0, 0]);
    auto_technique(&mut fifteen);
    assert_eq!(
        fifteen.iter().map(|s| s.techniques).collect::<Vec<_>>(),
        [[0; 6]; 6]
    );
}

/// Cost 4, clock 1000, now 3000: 2000 ms over 200 is 10 points, which cost
/// 40 and add 20 to the trained skill.
#[test]
fn a_companion_converts_by_the_clock() {
    let mut c = trainee(400, 0);
    c.clock = 1000;
    let out = convert_companion(3000, &mut c);
    assert_eq!(out.points, 10);
    assert_eq!((c.experience, c.clock, c.skills[0].points), (360, 3000, 20));
    assert_eq!(out.raised, None);
    // Too few points restart the clock; an inactive body does nothing.
    let mut poor = trainee(3, 0);
    convert_companion(9000, &mut poor);
    assert_eq!(poor.clock, 9000);
    let mut away = trainee(400, 0);
    away.body_state = 0x8 | ACTIVE;
    assert_eq!(convert_companion(9000, &mut away), Conversion::default());
    assert_eq!(away.clock, 0);
    // A clock behind now pays nothing.
    let mut early = trainee(400, 0);
    early.clock = 5000;
    assert_eq!(convert_companion(4000, &mut early).points, 0);
    assert_eq!(early.clock, 5000);
}

/// From 4990, five points reach 5000: combat skill 95 stops at 100, courage
/// rises to 10, the AI factors are 1 and 0.1, and total level 1 picks
/// Armour's technique 2.
#[test]
fn a_companion_level_raises_skill_and_courage() {
    let mut c = trainee(20, 4990);
    c.combat_skill = 95.0;
    let out = convert_companion(1000, &mut c);
    assert_eq!(out.points, 5);
    assert_eq!(out.raised, Some(0));
    assert_eq!(c.skills[0].points, 5000);
    assert_eq!((c.combat_skill, c.courage), (100.0, 10.0));
    assert_eq!(out.ai_factors, Some([1.0, 0.1]));
    assert_eq!(c.skills[2].techniques[0], 2);
    // No technique in training: the companion picks one instead.
    let mut idle = Learner {
        experience: 40,
        ..Learner::default()
    };
    let out = convert_companion(1000, &mut idle);
    assert_eq!((out.points, out.raised), (5, None));
    assert_eq!(idle.skills[2].techniques[0], 1);
}

/// The account-wide experience round-trips through the binary settings
/// store under its key; an empty store leaves the number alone.
#[test]
fn the_account_experience_round_trips_through_the_store() {
    use exa_formats::settings_store::SettingsStore;
    let mut store = SettingsStore::new();
    let xp = Experience {
        global: 12_345,
        ..Experience::default()
    };
    assert!(xp.save_global(&mut store).is_ok());
    assert_eq!(
        store.get(&GLOBAL_XP_KEY),
        Some(&global_xp_to_setting(12_345)[..])
    );
    assert_eq!(global_xp_to_setting(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
    let mut back = Experience::default();
    back.load_global(&store);
    assert_eq!(back.global, 12_345);
    // An empty store and an empty value keep the number.
    let mut kept = Experience {
        global: 7,
        ..Experience::default()
    };
    kept.load_global(&SettingsStore::new());
    assert_eq!(kept.global, 7);
    let mut empty = SettingsStore::new();
    empty.set(&GLOBAL_XP_KEY, &[]).unwrap();
    kept.load_global(&empty);
    assert_eq!(kept.global, 7);
}

/// The level-start hook makes the player's carried experience the account
/// pool and restarts both clocks at now.
#[test]
fn a_level_start_seeds_the_pool() {
    let mut xp = Experience {
        global: 900,
        global_clock: 1_000,
        player_clock: 2_000,
        pool: 0,
        ..Experience::default()
    };
    let mut player = Learner {
        experience: 640,
        ..Learner::default()
    };
    xp.start_level(5_000, &mut player);
    assert_eq!(
        (xp.pool, xp.global_clock, xp.player_clock),
        (640, 5_000, 5_000)
    );
    assert_eq!(xp.global, 900);
    assert_eq!(player.experience, 640);
}

/// The story installs six hooks, in the order its registration fills them.
#[test]
fn the_story_installs_six_hooks() {
    assert_eq!(STORY_HOOKS.len(), 6);
    assert_eq!(
        STORY_HOOKS,
        [
            Hook::NewGame,
            Hook::LevelEnter,
            Hook::LevelStart,
            Hook::Tick,
            Hook::Kill,
            Hook::Pickup
        ]
    );
}

/// The power's progress is divided as a signed number: a word above 2^31 is
/// negative, so a point does not turn it into half a million levels.
#[test]
fn a_negative_power_progress_converts_nothing() {
    let mut xp = Experience::default();
    let mut player = trainee(1000, 0);
    player.power_progress = 0x8000_0000;
    player.powers = vec![Power {
        id: 4,
        flags: POWER_IN_TRAINING | 5,
        blob: None,
    }];
    let out = xp.convert_player(400, &mut player);
    assert_eq!(out.points, 2);
    assert_eq!(player.power_progress, 0x8000_0008);
    assert_eq!(player.powers[0].flags, POWER_IN_TRAINING | 5);
}

/// The player pays 4 a point. Two points of 8 add two levels to the power
/// in training: level 5 becomes 7, and a level at 4095 becomes 0x1fff.
#[test]
fn the_player_converts_into_skill_and_power() {
    let mut xp = Experience::default();
    let mut player = trainee(1000, 0);
    player.powers = vec![
        Power {
            id: 3,
            flags: 0x5,
            blob: None,
        },
        Power {
            id: 4,
            flags: POWER_IN_TRAINING | 5,
            blob: None,
        },
    ];
    let out = xp.convert_player(400, &mut player);
    assert_eq!(out.points, 2);
    assert_eq!(player.experience, 992);
    assert_eq!(player.powers[0].flags, 0x5);
    assert_eq!(player.powers[1].flags, POWER_IN_TRAINING | 7);
    assert_eq!((player.power_progress, xp.player_clock), (0, 400));
    assert_eq!(player.skills[0].points, 4);
    let mut full = trainee(1000, 0);
    full.powers = vec![Power {
        id: 4,
        flags: POWER_IN_TRAINING | 0xfff,
        blob: None,
    }];
    Experience::default().convert_player(400, &mut full);
    assert_eq!(full.powers[0].flags, POWER_FULL);
    // A clock ahead of now converts nothing and stays.
    let mut later = Experience {
        player_clock: 1000,
        ..Experience::default()
    };
    assert_eq!(later.convert_player(0, &mut trainee(1000, 0)).points, 0);
    assert_eq!(later.player_clock, 1000);
}

/// Pool 100 at global 0 costs 4: 1000 ms is 5 points, global 10, pool 80.
/// At 25000 the pool empties. A short pool restarts the clock. A clock
/// ahead of now moves back by the negative count.
#[test]
fn the_account_pool_converts_and_stops_at_its_cap() {
    let mut xp = Experience {
        pool: 100,
        ..Experience::default()
    };
    xp.convert_global(1000);
    assert_eq!((xp.global, xp.pool, xp.global_clock), (10, 80, 1000));
    let mut capped = Experience {
        global: 26_000,
        pool: 100,
        ..Experience::default()
    };
    capped.convert_global(1000);
    assert_eq!((capped.global, capped.pool), (GLOBAL_CAP, 0));
    let mut short = Experience {
        global: 5000,
        pool: 7,
        ..Experience::default()
    };
    short.convert_global(777);
    assert_eq!(short.global_clock, 777);
    let mut ahead = Experience {
        pool: 100,
        global_clock: 1000,
        ..Experience::default()
    };
    ahead.convert_global(0);
    assert_eq!((ahead.global, ahead.pool, ahead.global_clock), (0, 100, 0));
}

/// A new kind of rarity tier 3 earns 100: the pool 100, each learner 400.
/// The same kind again, another mode, or a kind without a model earn 0. A
/// kill of rank 5 earns 500.
#[test]
fn new_items_and_kills_earn_points() {
    let mut xp = Experience::default();
    let mut player = Learner::default();
    let mut party = vec![Learner::default()];
    let got = xp.discover_item(STORY_MODE, Some(&mut player), &mut party, 77, |_| Some(3));
    assert_eq!(got, 100);
    assert_eq!(
        (xp.pool, player.experience, party[0].experience),
        (100, 400, 400)
    );
    assert_eq!(xp.seen, [77]);
    let again = xp.discover_item(STORY_MODE, Some(&mut player), &mut party, 77, |_| Some(3));
    assert_eq!(again, 0);
    assert_eq!(
        xp.discover_item(1, Some(&mut player), &mut party, 78, |_| Some(3)),
        0
    );
    assert_eq!(
        xp.discover_item(STORY_MODE, None, &mut party, 78, |_| Some(3)),
        0
    );
    assert_eq!(
        xp.discover_item(STORY_MODE, Some(&mut player), &mut party, 79, |_| None),
        0
    );
    assert_eq!(xp.seen, [77]);
    assert_eq!(xp.discover_kill(&mut player, &mut party, 0x5abc), 500);
    assert_eq!(player.experience, 2400);
    assert_eq!(rarity_points(9), 2000);
}

struct Grid {
    width: i32,
    height: i32,
    cell_width: i32,
    hidden: Vec<u32>,
}

impl ExploreGrid for Grid {
    fn tracks(&self) -> bool {
        true
    }
    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
    fn sector_word(&self, _cell: u32) -> Option<i32> {
        Some(1)
    }
    fn sector_open(&self, sector: i32) -> bool {
        sector == 0
    }
    fn cell_width(&self) -> i32 {
        self.cell_width
    }
    fn cell_flags(&self, cell: u32) -> u32 {
        if self.hidden.contains(&cell) {
            0
        } else {
            DISCOVERABLE
        }
    }
}

/// On a 10 × 10 grid the origin is cell (5, 5): the 5 × 5 cells from 3 to
/// 7 count once, one of them hidden, 24 cells and 240 points. Rows of a
/// 64-cell grid are two words. At x 2400 the cell is 9, cut to columns 7
/// and 8. Cells round down: x −300 falls in −1 and z 1100 in 1, where
/// rounding to the nearest would give 0 and 2.
#[test]
fn exploring_finds_each_cell_once() {
    let grid = Grid {
        width: 10,
        height: 10,
        cell_width: 64,
        hidden: vec![4 * 64 + 4],
    };
    let mut xp = Experience::default();
    xp.enter_level(*b"exanima01\0\0\0\0\0\0\0", 64, 10);
    assert_eq!(xp.maps[0].bits.len(), 20);
    let mut player = Learner::default();
    let found = xp.explore(&grid, 0.0, 0.0, &mut player, &mut []);
    assert_eq!(found, 24);
    assert_eq!((xp.pool, player.experience), (240, 960));
    assert_eq!(xp.maps[0].bits[10], 0xf8);
    assert_eq!(xp.maps[0].bits[8], 0xe8);
    assert_eq!(xp.explore(&grid, 0.0, 0.0, &mut player, &mut []), 0);
    let mut east = Experience::default();
    east.enter_level([1; 16], 64, 10);
    assert_eq!(east.explore(&grid, 2400.0, 0.0, &mut player, &mut []), 10);
    assert_eq!(east.maps[0].bits[10], 0x180);
    let mut down = Experience::default();
    down.enter_level([2; 16], 64, 10);
    down.explore(&grid, -300.0, 1100.0, &mut player, &mut []);
    // Columns 2..6 around 4, rows 4..8 around 6; (4, 4) is hidden.
    assert_eq!(down.maps[0].bits[2 * 4], 0x6c);
    assert_eq!(down.maps[0].bits[2 * 5], 0x7c);
    assert_eq!(down.maps[0].bits[2 * 3], 0);
    assert_eq!(down.maps[0].bits[2 * 8], 0x7c);
    assert_eq!(down.maps[0].bits[2 * 9], 0);
    // Entering a level again finds its map.
    xp.enter_level(*b"exanima01\0\0\0\0\0\0\0", 64, 10);
    assert_eq!((xp.maps.len(), xp.current), (1, Some(0)));
}

/// The chunk holds the seen kinds, then each map's name, size and bits. A
/// zero name recovers the maps before it; short or negative data fails.
#[test]
fn discovery_saves_and_loads() {
    let mut xp = Experience {
        seen: vec![5, 9],
        ..Experience::default()
    };
    xp.enter_level(*b"exanima01\0\0\0\0\0\0\0", 64, 2);
    xp.maps[0].bits = vec![1, 2, 3, 4];
    let bytes = xp.write_discovery();
    assert_eq!(bytes.len(), 4 + 8 + 4 + 16 + 8 + 16);
    let back = read_discovery(&bytes).unwrap();
    assert_eq!(
        (back.seen.clone(), back.maps.clone()),
        (xp.seen.clone(), xp.maps.clone())
    );
    assert!(!back.recovered);
    let mut cut = bytes.clone();
    cut[12] = 2;
    cut.extend([0u8; 16]);
    let recovered = read_discovery(&cut).unwrap();
    assert!(recovered.recovered);
    assert_eq!(recovered.maps.len(), 1);
    assert_eq!(
        read_discovery(&bytes[..bytes.len() - 1]),
        Err(DiscoveryError::Truncated)
    );
    assert_eq!(
        read_discovery(&(-1i32).to_le_bytes()),
        Err(DiscoveryError::Negative)
    );
    let mut fresh = Experience::default();
    fresh.restore(back);
    assert_eq!(fresh.seen, [5, 9]);
    fresh.begin();
    assert!(fresh.seen.is_empty() && fresh.maps.is_empty());
}

/// A companion joins once and starts its clock; leaving swaps in the last.
#[test]
fn companions_join_and_leave() {
    let mut list = vec![1, 2, 3];
    let mut c = Learner::default();
    assert!(!join(&mut list, 2, &mut c, 500));
    assert!(join(&mut list, 4, &mut c, 500));
    assert_eq!(c.clock, 500);
    assert!(leave(&mut list, &1));
    assert_eq!(list, [4, 2, 3]);
    assert!(!leave(&mut list, &9));
}

/// Three levels and combat skill 50: 2 + 1.2 + 1 = 4.2, quartered 1.05.
/// With weapon hundredths 2.67 at reach 120 and a scale of 1.5:
/// 4.2 + 6.408 + 2.
#[test]
fn the_fighting_estimate() {
    let skills = skills_with([5000, 10_000, 0, 0, 0, 0]);
    let alone = fighting_estimate(&skills, 50.0, None, 1.0);
    assert_eq!(alone.to_bits(), 0x3f86_6666);
    let mut stats = StatBytes::default();
    for (byte, value) in [
        (at::IMPACT, 127),
        (at::SLASH, 50),
        (at::CRUSH, 20),
        (at::PIERCE, 40),
        (at::THRUST, 30),
    ] {
        stats.set(byte, value);
    }
    let armed = fighting_estimate(&skills, 50.0, Some((&stats, 120.0)), 1.5);
    assert_eq!(armed.to_bits(), 0x4149_ba5f);
}

/// A tick explores, then converts the pool, the player and each companion.
#[test]
fn a_tick_runs_every_step() {
    let grid = Grid {
        width: 10,
        height: 10,
        cell_width: 64,
        hidden: vec![],
    };
    let mut xp = Experience::default();
    xp.enter_level([3; 16], 64, 10);
    let mut player = trainee(0, 0);
    let mut party = vec![trainee(0, 0)];
    party[0].clock = 1;
    let tick = xp.tick(1001, &grid, 0.0, 0.0, &mut player, &mut party);
    assert_eq!(tick.cells, 25);
    assert_eq!(tick.player.points, 5);
    assert_eq!(tick.companions[0].points, 5);
    assert_eq!(xp.global, 10);
    assert_eq!((player.experience, party[0].experience), (980, 980));
}
