use std::path::Path;

extern crate pacman_repo_tools as prt;
use prt::srcinfo::parse_srcinfo_dir;
use prt::srcinfo::walk_srcinfo_files;

fn main() {
	let args: Vec<_> = std::env::args().collect();

	println!("Searching in {:?}", &args);

	for entry in walk_srcinfo_files(Path::new(&args[1])) {
		let entry = entry.unwrap();
		println!("{}", entry.path().display());
	}

	for (name, package) in parse_srcinfo_dir(Path::new(&args[1])).unwrap().into_iter() {
		println!("{}: {:?}", name, package.version);
	}
}
