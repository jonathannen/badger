use crate::GraphArgs;
use super::{CliResult, Context};

pub fn run(_ctx: &Context, args: GraphArgs) -> CliResult {
    Err(format!(
        "graph not yet implemented (entry: {}, format: {:?})",
        args.entry.display(),
        args.format as u8
    )
    .into())
}
