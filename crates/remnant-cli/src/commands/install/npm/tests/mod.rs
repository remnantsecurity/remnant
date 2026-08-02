use super::*;

#[test]
fn resolve_args_default_to_bare_package_lock_only_install() {
    assert_eq!(
        build_resolve_args(Vec::new()),
        vec!["install", "--package-lock-only"]
    );
}

#[test]
fn resolve_args_prepend_install_and_package_lock_only_to_extra_args() {
    assert_eq!(
        build_resolve_args(vec![String::from("esbuild")]),
        vec!["install", "--package-lock-only", "esbuild"]
    );
}
