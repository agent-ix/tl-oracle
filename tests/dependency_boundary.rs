use std::process::Command;

/// TC-175, FR-043-AC-1: the resolved normal dependency graph cannot self-oracle.
#[test]
fn tc_175_dependency_graph_excludes_production_semantics() {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["tree", "--offline", "--edges", "normal", "--prefix", "none"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree must run for the dependency boundary gate");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("cargo tree output is UTF-8");
    assert!(tree.lines().any(|line| line.starts_with("tl-syntax ")));
    for forbidden in ["tl-mltl ", "tl-rewrite "] {
        assert!(
            !tree.lines().any(|line| line.starts_with(forbidden)),
            "forbidden dependency {forbidden} is present"
        );
    }
}
