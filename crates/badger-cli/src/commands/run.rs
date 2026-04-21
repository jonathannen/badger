use crate::RunArgs;
use super::{CliResult, Context};

pub fn run(_ctx: &Context, args: RunArgs) -> CliResult {
    Err(format!(
        "run not yet implemented (entry: {}, program args: {:?})",
        args.entry.display(),
        args.program_args
    )
    .into())
}
