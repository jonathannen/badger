use crate::BuildArgs;
use super::{CliResult, Context};

pub fn run(_ctx: &Context, args: BuildArgs) -> CliResult {
    Err(format!(
        "build not yet implemented (entry: {}, profile: {})",
        args.entry.display(),
        args.profile
    )
    .into())
}
