use crate::CheckArgs;
use super::{CliResult, Context};

pub fn run(_ctx: &Context, args: CheckArgs) -> CliResult {
    Err(format!("check not yet implemented (entry: {})", args.entry.display()).into())
}
