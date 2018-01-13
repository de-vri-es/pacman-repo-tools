use std::path::Path;

extern crate pacman_repo_tools as prt;
extern crate slice_tracker;

use prt::srcinfo::parse_srcinfo_dir;
use prt::srcinfo::walk_srcinfo_files;

use slice_tracker::{SliceTracker, SourceLocation};
type SourceTracker<'a> = SliceTracker<'a, str, SourceLocation<'a, str>>;

fn main() {
	let args: Vec<_> = std::env::args().collect();

	println!("Searching in {:?}", &args);
	let pool = SourceTracker::default();

	for entry in walk_srcinfo_files(Path::new(&args[1])) {
		let entry = entry.unwrap();
		println!("{}", entry.path().display());
	}

	for (name, package) in parse_srcinfo_dir(&pool, Path::new(&args[1])).unwrap().into_iter() {
		println!("{}: {:?}", name, package.version());
	}
}
