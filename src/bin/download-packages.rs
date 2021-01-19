use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;

use pacman_repo_tools::db::{read_db_dir, DatabasePackage};

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
	let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
	let runtime = match runtime {
		Ok(x) => x,
		Err(e) => {
			eprintln!("Error: failed to initialize tokio runtime: {}", e);
			std::process::exit(1);
		},
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
	let packages = index_packages_by_name(&repositories);

	let targets = if options.dependencies {
		let resolver = DependencyResolver::new(&packages);
		resolver.resolve(&targets)?
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
		let url = reqwest::Url::parse(url.as_ref()).map_err(|e| eprintln!("Invalid URL: {}: {}", url.as_ref(), e))?;
		let name = Path::new(url.path())
			.file_name()
			.ok_or_else(|| eprintln!("Could not determine file name from URL: {}", url))?
			.to_str()
			.ok_or_else(|| eprintln!("Invalid UTF-8 in URL: {}", url))?;

		if !names.insert(name.to_string()) {
			eprintln!("Error: duplicate repository name: {}", name);
			return Err(());
		}
		databases.push((name.to_string(), url));
	}

	for (name, url) in databases {
		let db_dir = directory.join(&name);
		download_database(&db_dir, url).await?;

		let packages = read_db_dir(&db_dir).map_err(|e| eprintln!("Error: {}", e))?;
		repositories.push((name, packages));
	}

	Ok(repositories)
}

fn index_packages_by_name(repositories: &[(String, Vec<DatabasePackage>)]) -> BTreeMap<&str, (&str, &DatabasePackage)> {
	use std::collections::btree_map::Entry;

	let mut index = BTreeMap::new();
	for (repo_name, packages) in repositories {
		for package in packages {
			match index.entry(package.name.as_str()) {
				Entry::Occupied(x) => {
					let (earlier_repo, _) = x.get();
					eprintln!(
						"Warning: package {} already encountered in {} repository, ignoring package from {} repository",
						package.name, earlier_repo, repo_name
					);
				},
				Entry::Vacant(entry) => {
					entry.insert((repo_name.as_str(), package));
				},
			}
		}
	}

	index
}

/// Create an index of virtual target names to concrete packages that provide the target.
fn index_providers<'a>(packages: &BTreeMap<&'a str, (&'a str, &'a DatabasePackage)>) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
	let mut index: BTreeMap<&'a str, BTreeSet<&'a str>> = BTreeMap::new();
	for (_repo, package) in packages.values() {
		index.entry(&package.name).or_default().insert(&package.name);
		for target in &package.provides {
			index.entry(&target.name).or_default().insert(&package.name);
		}
	}
	index
}

struct DependencyResolver<'a, 'b> {
	packages: &'b BTreeMap<&'a str, (&'a str, &'a DatabasePackage)>,
	providers: BTreeMap<&'a str, BTreeSet<&'a str>>,
	selected_packages: BTreeSet<&'a str>,
	provided_targets: BTreeSet<&'a str>,
}

impl<'a, 'b> DependencyResolver<'a, 'b> {
	pub fn new(packages: &'b BTreeMap<&'a str, (&'a str, &'a DatabasePackage)>) -> Self {
		Self {
			packages,
			providers: index_providers(&packages),
			selected_packages: BTreeSet::new(),
			provided_targets: BTreeSet::new(),
		}
	}

	pub fn resolve(mut self, targets: &[impl AsRef<str>]) -> Result<BTreeSet<&'a str>, ()> {
		let mut queue = BTreeSet::new();

		for target in targets {
			let target = target.as_ref();
			// First add all explicitly listed real packages.
			if let Some((_repo, package)) = self.packages.get(target) {
				self.add_package(package);
				for depend in &package.depends {
					queue.insert(depend.name.as_str());
				}
			// Add virtual targets to the queue to be resolved later.
			// They may already be provided by an explicitly listed package.
			} else {
				queue.insert(target);
			}
		}

		// Resolve targets in the queue until it is empty.
		while let Some(target) = pop_first(&mut queue) {
			// Ignore already-provided targets.
			// All explicitly listed packages have already been added,
			// so these are either virtual targets or dependencies.
			if self.provided_targets.contains(target) {
				continue;
			}

			let package = self.resolve_target(target)?;
			self.add_package(package);
			for depend in &package.depends {
				if !self.provided_targets.contains(depend.name.as_str()) {
					queue.insert(&depend.name);
				}
			}
		}

		Ok(self.selected_packages)
	}

	/// Add a package to the selection.
	fn add_package(&mut self, package: &'a DatabasePackage) {
		self.selected_packages.insert(&package.name);
		self.provided_targets.insert(&package.name);
		let provides = package.provides.iter().map(|x| x.name.as_str());
		self.provided_targets.extend(provides);
	}

	/// Choose a package for a target.
	///
	/// If the target is a concrete package, choose that.
	/// Otherwise, choose some implementation defined provider, if it exists.
	fn resolve_target(&self, target: &str) -> Result<&'a DatabasePackage, ()> {
		if let Some((_repo, package)) = self.packages.get(target) {
			Ok(package)
		} else {
			let provider = self.providers
				.get(target)
				.and_then(|x| x.iter().next())
				.ok_or_else(|| eprintln!("no provider found for target: {}", target))?;
			self.packages.get(provider)
				.map(|&(_repo, package)| package)
				.ok_or_else(|| eprint!("no such package: {}", provider))
		}
	}
}

fn pop_first<T: Copy + Ord>(set: &mut BTreeSet<T>) -> Option<T> {
	let value = *set.iter().next()?;
	set.take(&value)
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
	let mut stdin = process.stdin.take().ok_or_else(|| eprintln!("Error: failed to get stdin for bsdtar"))?;
	stdin
		.write_all(data)
		.await
		.map_err(|e| eprintln!("Error: failed to write to bsdtar stdin: {}", e))?;
	drop(stdin);

	// Wait for bsdtar to finish.
	let exit_status = process
		.wait()
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
	std::fs::create_dir_all(path).map_err(|e| eprintln!("Error: failed to create directiry {}: {}", path.display(), e))
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
		},
	}
}

/// Download a file over HTTP(S).
async fn download_file(url: reqwest::Url) -> Result<Vec<u8>, reqwest::Error> {
	let response = reqwest::get(url.clone()).await?.error_for_status()?;
	Ok(response.bytes().await?.to_vec())
}
