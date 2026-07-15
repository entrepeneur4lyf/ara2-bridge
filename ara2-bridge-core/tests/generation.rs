use ara2_bridge_core::{ApiGeneration, AraError, AssertCoordinator, FactoryInitialization};

#[test]
fn factories_keep_independent_generations_but_share_generation_cell() {
    let coordinator = AssertCoordinator::default();
    let mut first = FactoryInitialization::begin(ApiGeneration::V1Final, &coordinator).unwrap();
    let second = FactoryInitialization::begin(ApiGeneration::V23Final, &coordinator).unwrap();
    let third = FactoryInitialization::begin(ApiGeneration::V23Final, &coordinator).unwrap();

    assert_ne!(first.generation(), second.generation());
    assert_eq!(second.assert_address(), third.assert_address());
    assert_ne!(first.assert_address(), second.assert_address());
    assert_eq!(coordinator.active_count(ApiGeneration::V1Final), 1);
    assert_eq!(coordinator.active_count(ApiGeneration::V23Final), 2);

    first.uninitialize().unwrap();
    assert_eq!(coordinator.active_count(ApiGeneration::V1Final), 0);
    assert!(matches!(
        first.uninitialize(),
        Err(AraError::InvalidState("factory is not initialized"))
    ));

    drop(second);
    assert_eq!(coordinator.active_count(ApiGeneration::V23Final), 1);
    drop(third);
    assert_eq!(coordinator.active_count(ApiGeneration::V23Final), 0);
}

#[test]
fn raw_generation_round_trips_and_unknown_values_fail() {
    for (raw, generation) in [
        (1, ApiGeneration::V1Draft),
        (2, ApiGeneration::V1Final),
        (3, ApiGeneration::V2Draft),
        (4, ApiGeneration::V2Final),
        (5, ApiGeneration::V2xDraft),
        (6, ApiGeneration::V23Final),
    ] {
        assert_eq!(generation.as_raw(), raw);
        assert_eq!(ApiGeneration::try_from_raw(raw).unwrap(), generation);
    }
    assert!(matches!(
        ApiGeneration::try_from_raw(7),
        Err(AraError::InvalidArgument("unknown API generation"))
    ));
}

#[cfg(target_arch = "aarch64")]
#[test]
fn aarch64_rejects_legacy_generations() {
    let coordinator = AssertCoordinator::default();
    for generation in [
        ApiGeneration::V1Draft,
        ApiGeneration::V1Final,
        ApiGeneration::V2Draft,
    ] {
        assert!(!generation.supported_on_target());
        assert!(matches!(
            FactoryInitialization::begin(generation, &coordinator),
            Err(AraError::Unsupported(
                "API generation is unavailable on this target"
            ))
        ));
    }
}

#[cfg(not(target_arch = "aarch64"))]
#[test]
fn x86_family_accepts_every_released_generation() {
    for generation in ApiGeneration::ALL {
        assert!(generation.supported_on_target());
    }
}
