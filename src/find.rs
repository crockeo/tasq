use structopt::StructOpt;

use crate::db;

#[derive(Debug, StructOpt)]
pub struct Args {
    text: String,
}

pub async fn main(args: Args, database: db::Database) -> anyhow::Result<()> {
    // (1) find all active nodes in the database
    // (2) for each node's title, run a fuzzy find against the args
    // (3) come up with some kind of confidence value based on fuzzy search
    //   (a) be ok with insertions b/c someone could be writing parts of a word
    //   (b) don't be ok with a lot of deletions / replacements
    //       b/c that means they're probably typing something else
    //   (c) also simimlar thing for replacement cost
    // (4) filter to only show things which are above a certain level of confidence
    //
    // also maybe include configurability here, so folks can tune their own preferences
    //
    // follow-up: highlight the characters which triggered
    todo!();
}
