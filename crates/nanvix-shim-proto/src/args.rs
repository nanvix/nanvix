//! CLI argument parsing, compatible with containerd's Go-flag conventions.

/// Flags passed from containerd to the shim binary.
#[derive(Debug, Default)]
pub struct ShimArgs {
    pub debug: bool,
    pub namespace: String,
    pub id: String,
    pub socket: String,
    pub bundle: String,
    pub address: String,
    pub publish_binary: String,
}

/// The action the shim was invoked with.
#[derive(Debug)]
pub enum Action {
    Start(ShimArgs),
    Delete(ShimArgs),
    Run(ShimArgs),
    Version,
    Help,
}

/// Parse command-line arguments into an action and flags.
pub fn parse_args(args: &[String]) -> anyhow::Result<Action> {
    let mut shim_args = ShimArgs::default();
    let mut i = 1; // skip argv[0]
    let mut positional = Vec::new();
    let mut version = false;

    while i < args.len() {
        match args[i].as_str() {
            "-debug" => shim_args.debug = true,
            "-namespace" => {
                i += 1;
                shim_args.namespace = args.get(i).cloned().unwrap_or_default();
            }
            "-id" => {
                i += 1;
                shim_args.id = args.get(i).cloned().unwrap_or_default();
            }
            "-socket" => {
                i += 1;
                shim_args.socket = args.get(i).cloned().unwrap_or_default();
            }
            "-bundle" => {
                i += 1;
                shim_args.bundle = args.get(i).cloned().unwrap_or_default();
            }
            "-address" => {
                i += 1;
                shim_args.address = args.get(i).cloned().unwrap_or_default();
            }
            "-publish-binary" => {
                i += 1;
                shim_args.publish_binary = args.get(i).cloned().unwrap_or_default();
            }
            "-v" | "-version" | "--version" => version = true,
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    if version {
        return Ok(Action::Version);
    }

    match positional.first().map(|s| s.as_str()) {
        Some("start") => Ok(Action::Start(shim_args)),
        Some("delete") => Ok(Action::Delete(shim_args)),
        None => Ok(Action::Run(shim_args)),
        Some(_) => Ok(Action::Run(shim_args)),
    }
}
