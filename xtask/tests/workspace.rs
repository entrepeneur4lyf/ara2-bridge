use cargo_metadata::{DependencyKind, TargetKind};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn expected_packages_are_workspace_members() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let members: BTreeSet<_> = metadata.workspace_members.iter().collect();

    for expected in [
        "ara2-bridge-sys",
        "ara2-bridge-core",
        "ara2-bridge-plugin",
        "ara2-bridge-host",
        "ara2-bridge-companion",
        "ara2-bridge-testkit",
        "ara2-bridge",
        "xtask",
    ] {
        let package = metadata
            .packages
            .iter()
            .find(|package| package.name == expected)
            .unwrap_or_else(|| panic!("missing workspace package {expected}"));
        assert!(
            members.contains(&package.id),
            "missing workspace member {expected}"
        );
    }
}

#[test]
fn workspace_example_targets_have_unique_output_names() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let mut owners = BTreeMap::<&str, Vec<&str>>::new();

    for package in metadata.workspace_packages() {
        for target in &package.targets {
            if target.kind.contains(&TargetKind::Example) {
                owners
                    .entry(target.name.as_str())
                    .or_default()
                    .push(package.name.as_str());
            }
        }
    }

    let collisions: BTreeMap<_, _> = owners
        .into_iter()
        .filter(|(_, packages)| packages.len() > 1)
        .collect();
    assert!(
        collisions.is_empty(),
        "example output names must be workspace-unique: {collisions:?}"
    );
}

#[test]
fn workspace_dependency_edges_match_the_runtime_and_test_architecture() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let workspace_names: BTreeSet<_> = metadata
        .workspace_packages()
        .iter()
        .map(|package| package.name.as_str())
        .collect();

    let expected = [
        ("ara2-bridge-sys", &[][..]),
        ("ara2-bridge-core", &["ara2-bridge-sys"][..]),
        (
            "ara2-bridge-plugin",
            &[
                "ara2-bridge-companion",
                "ara2-bridge-core",
                "ara2-bridge-sys",
            ][..],
        ),
        (
            "ara2-bridge-host",
            &["ara2-bridge-core", "ara2-bridge-sys", "ara2-bridge-testkit"][..],
        ),
        (
            "ara2-bridge-companion",
            &[
                "ara2-bridge-sys",
                "ara2-bridge-core",
                "ara2-bridge-host",
                "ara2-bridge-plugin",
            ][..],
        ),
        (
            "ara2-bridge-testkit",
            &[
                "ara2-bridge-sys",
                "ara2-bridge-core",
                "ara2-bridge-plugin",
                "ara2-bridge-host",
                "ara2-bridge-companion",
            ][..],
        ),
        (
            "ara2-bridge",
            &[
                "ara2-bridge-sys",
                "ara2-bridge-core",
                "ara2-bridge-plugin",
                "ara2-bridge-host",
                "ara2-bridge-companion",
                "ara2-bridge-testkit",
            ][..],
        ),
        ("xtask", &["ara2-bridge-sys", "ara2-bridge-testkit"][..]),
    ];

    for (package_name, expected_dependencies) in expected {
        let package = metadata
            .packages
            .iter()
            .find(|package| package.name == package_name)
            .unwrap();
        let actual: BTreeSet<_> = package
            .dependencies
            .iter()
            .filter(|dependency| workspace_names.contains(dependency.name.as_str()))
            .map(|dependency| dependency.name.as_str())
            .collect();
        let expected: BTreeSet<_> = expected_dependencies.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "unexpected local edges for {package_name}"
        );
    }

    let testkit = metadata
        .packages
        .iter()
        .find(|package| package.name == "ara2-bridge-testkit")
        .unwrap();
    assert!(
        testkit
            .dependencies
            .iter()
            .all(|dependency| dependency.name != "ara2-bridge"),
        "testkit must never depend on the facade"
    );
}

#[test]
fn host_runtime_has_no_normal_dependency_on_plugin_or_testkit() {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    let host = metadata
        .packages
        .iter()
        .find(|package| package.name == "ara2-bridge-host")
        .unwrap();
    let forbidden = host
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == DependencyKind::Normal)
        .filter(|dependency| {
            matches!(
                dependency.name.as_str(),
                "ara2-bridge-plugin" | "ara2-bridge-testkit"
            )
        })
        .map(|dependency| dependency.name.as_str())
        .collect::<Vec<_>>();
    assert!(
        forbidden.is_empty(),
        "host runtime has forbidden normal dependencies: {forbidden:?}"
    );
}
