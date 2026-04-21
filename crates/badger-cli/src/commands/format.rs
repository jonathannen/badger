use crate::FormatArgs;
use super::{CliResult, Context};

pub fn run(_ctx: &Context, args: FormatArgs) -> CliResult {
    Err(format!(
        "format not yet implemented (paths: {:?}, check: {})",
        args.paths, args.check
    )
    .into())
}
