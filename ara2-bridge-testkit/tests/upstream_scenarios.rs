use ara2_bridge_testkit::scenarios::upstream_scenarios;
use std::collections::BTreeSet;

const REQUIRED: &[&str] = &[
    "property-updates",
    "content-updates",
    "content-reading",
    "audio-modification-cloning",
    "full-archive",
    "split-partial-archives",
    "drag-drop-import",
    "playback-rendering",
    "playback-rendering-time-stretch",
    "editor-view",
    "processing-algorithms",
    "audio-file-chunk-load",
    "audio-file-chunk-save",
    "basic-document",
    "ara1-persistence",
    "ara23-dirtiness",
    "extension-role-combinations",
    "poisoning",
    "controller-first-teardown",
    "companion-first-teardown",
];

#[test]
fn every_required_scenario_has_a_runner_and_executes_without_skips() {
    let scenarios = upstream_scenarios();
    let actual = scenarios
        .iter()
        .map(|scenario| scenario.name)
        .collect::<BTreeSet<_>>();
    let expected = REQUIRED.iter().copied().collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
    assert!(missing.is_empty(), "missing scenario runners: {missing:?}");
    assert_eq!(actual.len(), scenarios.len(), "duplicate scenario names");

    for scenario in scenarios {
        assert!(!scenario.required_capabilities.is_empty());
        let report = (scenario.run)()
            .unwrap_or_else(|error| panic!("scenario {} failed: {error}", scenario.name));
        assert_eq!(report.name(), scenario.name);
        assert!(report.operations() > 0);
        assert!(report.assertions() > 0);
        assert!(report.expected_calls() > 0);
        assert_eq!(report.skip_count(), 0);
    }
}
