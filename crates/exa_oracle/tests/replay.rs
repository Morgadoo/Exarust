//! Replays the recorded cases of the original's functions against the
//! Rust ports. Needs a directory of case files (recorded outside the
//! repository), so it is ignored by default:
//!
//! `EXA_ORACLE_CASES=<dir> cargo test -p exa_oracle --test replay -- --ignored --nocapture`

use exa_oracle::{Outcome, replay_dir};

#[test]
#[ignore = "needs EXA_ORACLE_CASES, a directory of recorded case files"]
fn every_recorded_case_matches_its_port() {
    let Some(dir) = std::env::var_os("EXA_ORACLE_CASES") else {
        eprintln!("EXA_ORACLE_CASES is not set; nothing to replay");
        return;
    };
    let results = replay_dir(std::path::Path::new(&dir)).expect("case directory reads");
    let mut failed = 0;
    for (name, outcome) in &results {
        match outcome {
            Outcome::Identical { cases, nan_payload } => {
                println!(
                    "{name}: identical over {cases} cases ({nan_payload} differ only in a NaN's bits)"
                )
            }
            Outcome::Mismatch {
                case,
                cases,
                differing,
                detail,
            } => {
                failed += 1;
                println!(
                    "{name}: MISMATCH in {differing} of {cases} cases; first is case {case}\n{detail}"
                );
            }
            Outcome::NoPort => println!("{name}: no port registered"),
            Outcome::BadLength { expected, got } => {
                failed += 1;
                println!("{name}: the port wrote {got} bytes, the original {expected}");
            }
        }
    }
    assert!(!results.is_empty(), "no case files under the directory");
    assert_eq!(failed, 0, "{failed} functions differ from the original");
}
