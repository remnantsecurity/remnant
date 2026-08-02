pub(super) fn real_lockfile_fixture() -> Vec<u8> {
    std::fs::read(fixture_path("package-lock.v3.real.json"))
        .expect("lockfile fixture should be readable")
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/commands/install/lockfile/tests/fixtures")
        .join(name)
}
