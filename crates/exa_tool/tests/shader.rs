//! The `shader` command against the installed shader library. Skips without
//! an installation.

use std::process::Command;

fn installed() -> bool {
    exa_rpk::locate_install(None).is_some()
}

#[test]
fn prints_the_original_program_for_a_material() {
    if !installed() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_exa-tool"))
        .args([
            "shader",
            "common.rml",
            "cobweb02",
            "--lights",
            "2",
            "--vertex-colors",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("// cobweb02 from common.rml: shading_f"));
    assert!(text.contains("// ---- vertex: shading_v"));
    assert!(text.contains("// ---- fragment: shading_f"));
    assert!(text.contains("void main()"));
    assert!(!text.contains("{?"));
    assert!(!text.contains("{%"));
    assert!(!text.contains("{$"));
}

#[test]
fn lists_the_warmup_variants() {
    if !installed() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_exa-tool"))
        .args(["shader", "--warmup"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("row  1 lights 0"));
    assert!(text.contains("row 29 lights 6"));
}

#[test]
fn lists_the_library_entries() {
    if !installed() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_exa-tool"))
        .args(["shader", "--list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.lines().any(|line| line == "shadeopt_f"));
    assert!(text.lines().any(|line| line == "ambient"));
}
