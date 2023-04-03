use std::str;

use structopt::StructOpt;

use crate::db;

const INSERT_COST: usize = 0;
const DELETE_COST: usize = 1;
const REPLACE_COST: usize = 1;
const MAX_DISTANCE: usize = 5;

#[derive(Debug, StructOpt)]
pub struct Args {
    text: String,
}

pub async fn main(args: Args, database: db::Database) -> anyhow::Result<()> {
    let candidates = find_candidates(&args.text, &database).await?;
    for (candidate, _) in candidates.into_iter() {
        println!("{} {}", candidate.title, candidate.id);
    }
    Ok(())
}

pub async fn find_candidates(search_string: &str, database: &db::Database) -> anyhow::Result<Vec<(db::Node, usize)>> {
    // (1) find all active nodes in the database
    let active_nodes = database.get_active_nodes().await?;

    // (2) for each node's title, run a fuzzy find against the args
    let mut candidates = Vec::new();
    for active_node in active_nodes.into_iter() {
        let node = database.get_node(active_node).await?;

        // (3) come up with some kind of confidence value based on fuzzy search
        //   (a) be ok with insertions b/c someone could be writing parts of a word
        //   (b) don't be ok with a lot of deletions / replacements
        //       b/c that means they're probably typing something else
        //   (c) also simimlar thing for replacement cost
        let distance = levenshtein_distance(search_string, &node.title);

        // (4) filter to only show things which are above a certain level of confidence
        if distance < MAX_DISTANCE {
            candidates.push((node, distance));
        }
    }

    Ok(candidates)
}

fn levenshtein_distance(from: &str, to: &str) -> usize {
    let from_chars: Vec<char> = from.chars().collect();
    let to_chars: Vec<char> = to.chars().collect();

    let mut last_row = vec![0; to_chars.len() + 1];
    let mut this_row = vec![0; to_chars.len() + 1];

    for i in 0..last_row.len() {
        last_row[i] = i * INSERT_COST
    }

    for i in 0..from_chars.len() {
        this_row[0] = i + 1;

        for j in 0..to_chars.len() {
            let deletion_cost = last_row[j + 1] + DELETE_COST;
            let insertion_cost = this_row[j] + INSERT_COST;
	    let replacement_cost;
	    if from_chars[i] == to_chars[j] {
		replacement_cost = last_row[j];
	    } else {
		replacement_cost = last_row[j] + REPLACE_COST;
	    }

	    this_row[j + 1] = deletion_cost.min(insertion_cost).min(replacement_cost)
        }

	let tmp = last_row;
	last_row = this_row;
	this_row = tmp;
    }

    return last_row[to_chars.len()]
}
