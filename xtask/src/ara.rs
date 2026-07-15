//! ARA artifact generation and verification command router.

/// Runs an ARA maintainer subcommand.
pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("bindings") => run_bindings(args),
        Some("generate") => run_generate(args),
        Some("fixtures") => run_fixtures(args),
        Some("fuzz-corpus") => run_fuzz_corpus(args),
        Some("probe-core") => run_core_probe(args),
        Some("companion-probe") => run_companion_probe(args),
        Some("coverage") => run_coverage(args),
        Some("provenance") => run_provenance(args),
        Some("host-dispatch") => run_host_dispatch(args),
        Some("plugin-dispatch") => run_plugin_dispatch(args),
        Some(command) => Err(format!("unknown ARA command: {command}")),
        None => Err("expected an ARA command".to_owned()),
    }
}

fn run_fuzz_corpus(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let action = args.next();
    if action.as_deref() == Some("--help") {
        if args.next().is_some() {
            return Err("unexpected fuzz-corpus arguments".to_owned());
        }
        return Ok(());
    }
    let mode = parse_mode(action.as_deref(), "fuzz-corpus")?;
    if args.next().is_some() {
        return Err("unexpected fuzz-corpus arguments".to_owned());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    crate::fuzz_corpus::generate(root, mode).map_err(|error| error.to_string())
}

fn run_coverage(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let action = args.next();
    if action.as_deref() == Some("--help") {
        if args.next().is_some() {
            return Err("unexpected coverage arguments".to_owned());
        }
        return Ok(());
    }
    let mode = parse_mode(action.as_deref(), "coverage")?;
    if args.next().is_some() {
        return Err("unexpected coverage arguments".to_owned());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    crate::coverage::generate(root, mode).map_err(|error| error.to_string())
}

fn run_companion_probe(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let component = args
        .next()
        .ok_or_else(|| "companion-probe requires a component or --help".to_owned())?;
    if component == "--help" {
        return Ok(());
    }
    if !matches!(component.as_str(), "clap" | "vst3" | "audio-unit-v2") {
        return Err(format!("unknown companion component: {component}"));
    }
    let action = args.next().ok_or_else(|| {
        "companion-probe requires --emit, --import-dir, or --check-all".to_owned()
    })?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    let result = match action.as_str() {
        "--emit" => {
            let output = args
                .next()
                .ok_or_else(|| "--emit requires an output path".to_owned())?;
            if args.next().as_deref() != Some("--target") {
                return Err("--emit requires --target <triple>".to_owned());
            }
            let target = args
                .next()
                .ok_or_else(|| "--target requires a target triple".to_owned())?;
            if args.next().is_some() {
                return Err("unexpected companion-probe arguments".to_owned());
            }
            crate::companion_probe::emit(root, &component, std::path::Path::new(&output), &target)
        }
        "--import-dir" => {
            let directory = args
                .next()
                .ok_or_else(|| "--import-dir requires a directory".to_owned())?;
            if args.next().is_some() {
                return Err("unexpected companion-probe arguments".to_owned());
            }
            crate::companion_probe::import_dir(root, &component, std::path::Path::new(&directory))
        }
        "--check-all" => {
            if args.next().is_some() {
                return Err("unexpected companion-probe arguments".to_owned());
            }
            crate::companion_probe::check_all(root, &component)
        }
        _ => {
            return Err("companion-probe requires --emit, --import-dir, or --check-all".to_owned())
        }
    };
    result.map_err(|error| error.to_string())
}

fn run_host_dispatch(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mode = parse_mode(args.next().as_deref(), "host-dispatch")?;
    if args.next().is_some() {
        return Err("unexpected host-dispatch arguments".to_owned());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    crate::host_dispatch::generate(root, mode).map_err(|error| error.to_string())
}

fn run_plugin_dispatch(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mode = parse_mode(args.next().as_deref(), "plugin-dispatch")?;
    if args.next().is_some() {
        return Err("unexpected plugin-dispatch arguments".to_owned());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    crate::plugin_dispatch::generate(root, mode).map_err(|error| error.to_string())
}

fn run_fixtures(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mode = parse_mode(args.next().as_deref(), "fixtures")?;
    if args.next().as_deref() != Some("--set") {
        return Err("fixtures requires --write|--check --set <name>".to_owned());
    }
    let set = args
        .next()
        .ok_or_else(|| "fixtures requires --set <name>".to_owned())?;
    if args.next().is_some() {
        return Err("unexpected fixtures arguments".to_owned());
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    crate::fixtures::generate(root, mode, &set).map_err(|error| error.to_string())
}

fn run_generate(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mode = parse_mode(args.next().as_deref(), "generate")?;
    if args.next().is_some() {
        return Err("unexpected generate arguments".to_owned());
    }
    crate::bindings::generate(mode).map_err(|error| error.to_string())?;
    crate::compatibility::generate(mode).map_err(|error| error.to_string())?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    crate::core_probe::generate_support(root, mode).map_err(|error| error.to_string())
}

fn run_core_probe(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let action = args
        .next()
        .ok_or_else(|| "probe-core requires --emit, --import-dir, or --check-all".to_owned())?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    match action.as_str() {
        "--emit" => {
            let output = args
                .next()
                .ok_or_else(|| "--emit requires an output path".to_owned())?;
            if args.next().as_deref() != Some("--target-family") {
                return Err("--emit requires --target-family <family>".to_owned());
            }
            let family = args
                .next()
                .ok_or_else(|| "--target-family requires a family".to_owned())?;
            if args.next().is_some() {
                return Err("unexpected probe-core arguments".to_owned());
            }
            crate::core_probe::emit(root, std::path::Path::new(&output), &family)
                .map_err(|error| error.to_string())
        }
        "--import-dir" => {
            let directory = args
                .next()
                .ok_or_else(|| "--import-dir requires a directory".to_owned())?;
            if args.next().is_some() {
                return Err("unexpected probe-core arguments".to_owned());
            }
            crate::core_probe::import_dir(root, std::path::Path::new(&directory))
                .map_err(|error| error.to_string())
        }
        "--check-all" => {
            if args.next().is_some() {
                return Err("unexpected probe-core arguments".to_owned());
            }
            crate::core_probe::check_all(root).map_err(|error| error.to_string())
        }
        _ => Err("probe-core requires --emit, --import-dir, or --check-all".to_owned()),
    }
}

fn run_bindings(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mode = parse_mode(args.next().as_deref(), "bindings")?;
    if args.next().is_some() {
        return Err("unexpected bindings arguments".to_owned());
    }
    crate::bindings::generate(mode).map_err(|error| error.to_string())
}

fn parse_mode(value: Option<&str>, command: &str) -> Result<crate::Mode, String> {
    match value {
        Some("--check") => Ok(crate::Mode::Check),
        Some("--write") => Ok(crate::Mode::Write),
        _ => Err(format!("{command} requires --check or --write")),
    }
}

fn run_provenance(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mode = args
        .next()
        .ok_or_else(|| "provenance requires --check or --refresh".to_owned())?;
    let component = match args.next().as_deref() {
        Some("--component") => Some(
            args.next()
                .ok_or_else(|| "--component requires a name".to_owned())?,
        ),
        Some(_) => return Err("unexpected provenance arguments".to_owned()),
        None => None,
    };
    if args.next().is_some() {
        return Err("unexpected provenance arguments".to_owned());
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a workspace child");
    let manifest = root.join("sdk-provenance.toml");
    let result = match (mode.as_str(), component.as_deref()) {
        ("--check", Some(component)) => crate::provenance::verify_component(root, component),
        ("--refresh", Some(component)) => crate::provenance::refresh_component(root, component),
        ("--check", None) => crate::provenance::verify(root, manifest),
        ("--refresh", None) => crate::provenance::refresh(root, manifest),
        _ => return Err("provenance requires --check or --refresh".to_owned()),
    };
    result.map_err(|error| error.to_string())
}
