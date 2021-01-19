use structopt::StructOpt;
use structopt::clap::AppSettings;
use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, BTreeSet};

use pacman_repo_tools::db::{DatabasePackage, read_db_dir};

#[derive(StructOpt)]
#[structopt(setting = AppSettings::ColoredHelp)]
#[structopt(setting = AppSettings::UnifiedHelpMessage)]
#[structopt(setting = AppSettings::DeriveDisplayOrder)]
struct Options {
	/// Download the package by name.
	#[structopt(long = "package", short)]
	#[structopt(value_name = "NAME")]
	packages: Vec<String>,

	/// Read packages to download from a file, one package name per line.
	#[structopt(long = "package-file", short = "f")]
	#[structopt(value_name = "PATH")]
	package_files: Vec<PathBuf>,

	/// Download all dependencies too.
	#[structopt(long, short)]
	dependencies: bool,

	/// A repository to download packages from (specify the URL for the database archive).
	#[structopt(long = "database", short = "r")]
	#[structopt(value_name = "URL.db.tar.*")]
	databases: Vec<String>,

	/// Read repository database URLs from a file, one database URL per line.
	#[structopt(long = "database-file")]
	#[structopt(value_name = "PATH")]
	database_files: Vec<PathBuf>,

	/// Download packages to this folder.
	#[structopt(long)]
	#[structopt(value_name = "DIRECTORY")]
	#[structopt(default_value = "packages")]
	package_dir: PathBuf,

	/// Download repository databases to this folder.
	#[structopt(long)]
	#[structopt(value_name = "DIRECTORY")]
	#[structopt(default_value = "db")]
	database_dir: PathBuf,
}

fn main() {
	let runtime = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build();
	let runtime = match runtime {
		Ok(x) => x,
		Err(e) => {
			eprintln!("Error: failed to initialize tokio runtime: {}", e);
			std::process::exit(1);
		}
	};
	runtime.block_on(async {
		if do_main(Options::from_args()).await.is_err() {
			std::process::exit(1);
		}
	})
}

async fn do_main(options: Options) -> Result<(), ()> {
	let targets = read_files_to_vec(options.packages, &options.package_files)?;
	let databases = read_files_to_vec(options.databases, &options.database_files)?;

	if targets.is_empty() {
		eprintln!("Error: need atleast one package to download");
		return Err(());
	}

	if databases.is_empty() {
		eprintln!("Error: need atleast one repository database");
		return Err(());
	}

	let repositories = sync_dbs(&options.database_dir, &databases).await?;
	eprintln!("Building package index.");
	let packages = index_packages_by_name(&repositories);

	let targets = if options.dependencies {
		eprintln!("Building providers index.");
		let providers = index_providers(&packages);
		eprintln!("Building dependency index.");
		let depends = index_dependencies(&packages);

		let mut resolver = DependencyResolver::new(&depends, &providers);
		for target in &targets {
			resolver.add_target(target)?;
		}
		resolver.into_targets()
	} else {
		targets.iter().map(String::as_str).collect()
	};


	eprintln!("Packages to download: {:?}", targets);

	Ok(())
}

/// Read the lines of a list of files into a vector.
///
/// Leading and trailing whitespace of each line is trimmed.
/// Empty lines and lines that start with a '#' (after stripping) are skipped.
fn read_files_to_vec(initial: Vec<String>, paths: &[impl AsRef<Path>]) -> Result<Vec<String>, ()> {
	let mut result = initial;

	for path in paths {
		let path = path.as_ref();
		let buffer = std::fs::read(&path).map_err(|e| eprintln!("Error: failed to read {}: {}", path.display(), e))?;
		let buffer = String::from_utf8(buffer).map_err(|e| eprintln!("Error: invalid UTF-8 in {}: {}", path.display(), e))?;

		result.extend(buffer.lines().filter_map(|line| {
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') {
				None
			} else {
				Some(String::from(line))
			}
		}));
	}

	Ok(result)
}

/// Download and extract the given database files specified by the URLs to the given directory.
async fn sync_dbs(directory: impl AsRef<Path>, urls: &[impl AsRef<str>]) -> Result<Vec<(String, Vec<DatabasePackage>)>, ()> {
	let directory = directory.as_ref();

	let mut names = std::collections::BTreeSet::new();
	let mut databases = Vec::new();
	let mut repositories = Vec::new();

	// TODO: check for duplicate database names
	for url in urls {
		let url = reqwest::Url::parse(url.as_ref())
			.map_err(|e| eprintln!("Invalid URL: {}: {}", url.as_ref(), e))?;
		let name = Path::new(url.path())
			.file_name()
			.ok_or_else(|| eprintln!("Could not determine file name from URL: {}", url))?
			.to_str()
			.ok_or_else(|| eprintln!("Invalid UTF-8 in URL: {}", url))?;

		if !names.insert(name.to_string()) {
			eprintln!("Error: duplicate repository name: {}", name);
			return Err(())
		}
		databases.push((name.to_string(), url));
	}

	for (name, url) in databases {
		let db_dir = directory.join(&name);
		download_database(&db_dir, url).await?;

		let packages = read_db_dir(&db_dir)
			.map_err(|e| eprintln!("Error: {}", e))?;
		repositories.push((name, packages));
	}

	Ok(repositories)
}

fn index_packages_by_name(repositories: &[(String, Vec<DatabasePackage>)]) -> BTreeMap<&str, (&str, &DatabasePackage)> {
	use std::collections::btree_map::Entry;

	let mut index = BTreeMap::new();
	for (repo_name, packages) in repositories {
		for package in packages {
			match index.entry(package.desc.name.as_str()) {
				Entry::Occupied(x) => {
					let (earlier_repo, _) = x.get();
					eprintln!("Warning: package {} already encountered in {} repository, ignoring package from {} repository", package.desc.name, earlier_repo, repo_name);
				}
				Entry::Vacant(entry) => {
					entry.insert((repo_name.as_str(), package));
				}
			}
		}
	}

	index
}

/// Create an index of virtual target names to concrete packages that provide the target.
fn index_providers<'a>(packages: &BTreeMap<&'a str, (&'a str, &'a DatabasePackage)>) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
	let mut index: BTreeMap<&'a str, BTreeSet<&'a str>> = BTreeMap::new();
	for (_repo, package) in packages.values() {
		index.entry(&package.desc.name).or_default().insert(&package.desc.name);
		for target in &package.depends.provides {
			index.entry(&target.name).or_default().insert(&package.desc.name);
		}
	}
	index
}

fn index_dependencies<'a>(packages: &BTreeMap<&'a str, (&'a str, &'a DatabasePackage)>) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
	let mut index: BTreeMap<&'a str, BTreeSet<&'a str>> = BTreeMap::new();
	for (_repo, package) in packages.values() {
		let depends = index.entry(&package.desc.name).or_default();
		depends.extend(package.depends.depends.iter().map(|x| x.name.as_str()));
	}
	index
}

struct DependencyResolver<'a, 'b> {
	dependencies: &'b BTreeMap<&'a str, BTreeSet<&'a str>>,
	providers: &'b BTreeMap<&'a str, BTreeSet<&'a str>>,
	reachable: BTreeSet<&'a str>,
}

impl<'a, 'b> DependencyResolver<'a, 'b> {
	pub fn new(dependencies: &'b BTreeMap<&'a str, BTreeSet<&'a str>>, providers: &'b BTreeMap<&'a str, BTreeSet<&'a str>>) -> Self {
		Self {
			dependencies,
			providers,
			reachable: BTreeSet::new(),
		}
	}

	pub fn into_targets(self) -> BTreeSet<&'a str> {
		self.reachable
	}

	pub fn add_target(&mut self, target: &'a str) -> Result<(), ()> {
		if self.reachable.contains(target) {
			return Ok(());
		}

		let mut targets = BTreeSet::new();
		targets.insert(target);

		while !targets.is_empty() {
			let target = *targets.iter().next().unwrap();
			targets.remove(target);
			if let Some(depends) = self.dependencies.get(target) {
				targets.extend(depends.difference(&self.reachable));
				self.reachable.insert(target);
			} else if let Some(providers) = self.providers.get(target) {
				if let Some(provider) = providers.iter().next() {
					if let Some(depends) = self.dependencies.get(provider) {
						targets.extend(depends.difference(&self.reachable));
						self.reachable.insert(provider);
						continue;
					} else {
						eprintln!("Error: missing dependency information for package {} as provider for {}", provider, target);
						return Err(());
					}
				} else {
					eprintln!("Error: unknown package: {}", target);
					return Err(());
				}
			} else {
				eprintln!("Error: unknown package: {}", target);
				return Err(());
			}
		}

		Ok(())
	}
}

fn push_entry<K: Ord, V>(map: &mut BTreeMap<K, Vec<V>>, key: K, value: V) {
	use std::collections::btree_map::Entry;
	match map.entry(key) {
		Entry::Vacant(x) => {
			x.insert(vec![value]);
		},
		Entry::Occupied(mut x) => {
			x.get_mut().push(value);
		}
	}
}

/// Download and extract a database file.
async fn download_database(directory: &Path, url: reqwest::Url) -> Result<(), ()> {
	// TODO: Record modified time and/or ETag to avoid downloading without cause.
	eprintln!("Downloading {}", url);
	let database = download_file(url).await.map_err(|e| eprintln!("Error: {}", e))?;
	extract_archive(&directory, &database).await?;
	Ok(())
}

/// Extract an archive in a directory using bsdtar.
async fn extract_archive(directory: &Path, data: &[u8]) -> Result<(), ()> {
	use tokio::io::AsyncWriteExt;

	// Delete and re-create directory for extracting the archive.
	remove_dir_all(directory)?;
	make_dirs(directory)?;

	// Spawn bsdtar process.
	let mut process = tokio::process::Command::new("bsdtar")
		.args(&["xf", "-"])
		.current_dir(directory)
		.stdin(std::process::Stdio::piped())
		.spawn()
		.map_err(|e| eprintln!("Error: failed to run bsdtar: {}", e))?;

	// Write archive to standard input of bsdtar.
	let mut stdin = process.stdin.take()
		.ok_or_else(|| eprintln!("Error: failed to get stdin for bsdtar"))?;
	stdin.write_all(data)
		.await
		.map_err(|e| eprintln!("Error: failed to write to bsdtar stdin: {}", e))?;
	drop(stdin);

	// Wait for bsdtar to finish.
	let exit_status = process.wait()
		.await
		.map_err(|e| eprintln!("Error: failed to wait for bsdtar to exit: {}", e))?;

	// Check the exit status.
	if exit_status.success() {
		Ok(())
	} else {
		eprintln!("Error: bsdtar exitted with {}", exit_status);
		Err(())
	}
}

/// Create a directory and all parent directories as needed.
fn make_dirs(path: impl AsRef<Path>) -> Result<(), ()> {
	let path = path.as_ref();
	std::fs::create_dir_all(path)
		.map_err(|e| eprintln!("Error: failed to create directiry {}: {}", path.display(), e))
}

/// Recursively remove a directory and it's content.
fn remove_dir_all(path: impl AsRef<Path>) -> Result<(), ()> {
	let path = path.as_ref();
	match std::fs::remove_dir_all(path) {
		Ok(()) => Ok(()),
		Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(e) => {
			eprintln!("Error: failed to remove directory {} or it's content: {}", path.display(), e);
			Err(())
		}
	}
}

/// Download a file over HTTP(S).
async fn download_file(url: reqwest::Url) -> Result<Vec<u8>, reqwest::Error> {
	let response = reqwest::get(url.clone())
		.await?
		.bytes()
		.await?
		.to_vec();
	Ok(response)
}
