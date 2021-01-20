use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;

use pacman_repo_tools::db::{read_db_dir, DatabasePackage};
use pacman_repo_tools::parse::rpartition;

#[derive(StructOpt)]
#[structopt(setting = AppSettings::ColoredHelp)]
#[structopt(setting = AppSettings::UnifiedHelpMessage)]
#[structopt(setting = AppSettings::DeriveDisplayOrder)]
struct Options {
	/// Download the package by name.
	#[structopt(long, short)]
	#[structopt(value_name = "NAME")]
	pkg: Vec<String>,

	/// Read packages to download from a file, one package name per line.
	#[structopt(long, short = "f")]
	#[structopt(value_name = "PATH")]
	pkg_file: Vec<PathBuf>,

	/// Download all dependencies too.
	#[structopt(long)]
	no_deps: bool,

	/// A repository to download packages from (specify the URL for the database archive).
	#[structopt(long)]
	#[structopt(value_name = "URL.db")]
	db_url: Vec<String>,

	/// Read repository database URLs from a file, one database URL per line.
	#[structopt(long, short)]
	#[structopt(value_name = "PATH")]
	db_file: Vec<PathBuf>,

	/// Download packages to this folder.
	#[structopt(long, short = "o")]
	#[structopt(value_name = "DIRECTORY")]
	#[structopt(default_value = "packages")]
	pkg_dir: PathBuf,

	/// Download repository databases to this folder.
	#[structopt(long)]
	#[structopt(value_name = "DIRECTORY")]
	#[structopt(default_value = "db")]
	db_dir: PathBuf,
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
	let targets = read_files_to_vec(options.pkg, &options.pkg_file)?;
	let databases = read_files_to_vec(options.db_url, &options.db_file)?;

	if targets.is_empty() {
		eprintln!("Error: need atleast one package to download");
		return Err(());
	}

	if databases.is_empty() {
		eprintln!("Error: need atleast one repository database");
		return Err(());
	}

	let repositories = Repository::parse_urls(&databases)?;

	eprintln!("Syncing repository databases");
	let packages = sync_dbs(&options.db_dir, &repositories).await?;
	let packages = index_packages_by_name(&packages);

	let selected_packages = if options.no_deps {
		targets.iter().map(String::as_str).collect()
	} else {
		let resolver = DependencyResolver::new(&packages);
		resolver.resolve(&targets)?
	};

	eprintln!("Downloading packages");
	download_packages(&options.pkg_dir, &selected_packages, &packages).await?;

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

struct Repository {
	name: String,
	db_url: reqwest::Url,
}

impl Repository {
	/// Parse a list of repository URLs.
	///
	/// If different URLs refer to repositories with the same name,
	/// an error is returned.
	fn parse_urls(urls: &[impl AsRef<str>]) -> Result<Vec<Repository>, ()> {
		let mut names = BTreeSet::new();
		let mut repositories = Vec::with_capacity(urls.len());
		for url in urls {
			let repository: Repository = url.as_ref().parse()?;
			if !names.insert(repository.name.clone()) {
				eprintln!("Error: duplicate repository name: {}", repository.name);
				return Err(());
			}
			repositories.push(repository);
		}

		Ok(repositories)
	}
}

impl std::str::FromStr for Repository {
	type Err = ();

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		let db_url: reqwest::Url = input.parse().map_err(|e| eprintln!("Error: invalid URL: {}: {}", input, e))?;
		let name = rpartition(db_url.path(), '/').map(|(_, name)| name).unwrap_or(db_url.path());
		if name.is_empty() {
			eprintln!("Error: can not determine repository name from URL: {}", input);
			return Err(());
		}
		Ok(Self { name: name.into(), db_url })
	}
}

/// Download and extract the given database files specified by the URLs to the given directory.
async fn sync_dbs<'a>(directory: impl AsRef<Path>, repositories: &'a [Repository]) -> Result<Vec<(&'a Repository, Vec<DatabasePackage>)>, ()> {
	let directory = directory.as_ref();

	let mut repo_packages = Vec::new();

	for repo in repositories {
		let db_dir = directory.join(&repo.name);
		download_database(&db_dir, &repo.db_url).await?;

		let packages = read_db_dir(&db_dir).map_err(|e| eprintln!("Error: {}", e))?;
		repo_packages.push((repo, packages));
	}

	Ok(repo_packages)
}

fn index_packages_by_name<'a>(packages: &'a [(&'a Repository, Vec<DatabasePackage>)]) -> BTreeMap<&'a str, (&'a Repository, &'a DatabasePackage)> {
	use std::collections::btree_map::Entry;

	let mut index: BTreeMap<&str, (&Repository, &DatabasePackage)> = BTreeMap::new();
	for (repo, packages) in packages {
		for package in packages {
			match index.entry(package.name.as_str()) {
				Entry::Occupied(x) => {
					let (prev_repo, _) = x.get();
					eprintln!(
						"Warning: package {} already encountered in {} repository, ignoring package from {} repository",
						package.name, prev_repo.name, repo.name
					);
				},
				Entry::Vacant(entry) => {
					entry.insert((repo, package));
				},
			}
		}
	}

	index
}

/// Create an index of virtual target names to concrete packages that provide the target.
fn index_providers<'a>(packages: &BTreeMap<&'a str, (&'a Repository, &'a DatabasePackage)>) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
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
	packages: &'b BTreeMap<&'a str, (&'a Repository, &'a DatabasePackage)>,
	providers: BTreeMap<&'a str, BTreeSet<&'a str>>,
	selected_packages: BTreeSet<&'a str>,
	provided_targets: BTreeSet<&'a str>,
}

impl<'a, 'b> DependencyResolver<'a, 'b> {
	pub fn new(packages: &'b BTreeMap<&'a str, (&'a Repository, &'a DatabasePackage)>) -> Self {
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
			let provider = self
				.providers
				.get(target)
				.and_then(|x| x.iter().next())
				.ok_or_else(|| eprintln!("no provider found for target: {}", target))?;
			self.packages
				.get(provider)
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
async fn download_database(directory: &Path, url: &reqwest::Url) -> Result<(), ()> {
	// TODO: Record modified time and/or ETag to avoid downloading without cause.
	eprintln!("Downloading {}", url);
	let database = download_file(&url).await.map_err(|e| eprintln!("Error: {}", e))?;
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
	std::fs::create_dir_all(path).map_err(|e| eprintln!("Error: failed to create directory {}: {}", path.display(), e))
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
async fn download_file(url: &reqwest::Url) -> Result<Vec<u8>, reqwest::Error> {
	let response = reqwest::get(url.clone()).await?.error_for_status()?;
	Ok(response.bytes().await?.to_vec())
}

async fn download_packages(
	directory: &impl AsRef<Path>,
	selected: &BTreeSet<&str>,
	packages: &BTreeMap<&str, (&Repository, &DatabasePackage)>,
) -> Result<(), ()> {
	let directory = directory.as_ref();
	for pkg_name in selected {
		let (repository, package) = packages
			.get(pkg_name)
			.expect(&format!("selected package list contains unknown package: {}", pkg_name));
		download_package(directory, repository, package).await?;
	}
	Ok(())
}

async fn download_package(directory: impl AsRef<Path>, repository: &Repository, package: &DatabasePackage) -> Result<(), ()> {
	use std::io::Write;
	let directory = directory.as_ref();
	make_dirs(directory)?;

	let pkg_url = package_url(repository, package);
	let pkg_path = directory.join(&package.filename);
	if let Some(metadata) = stat(&pkg_path)? {
		if metadata.len() == package.compressed_size {
			// TODO: check sha256sum
			return Ok(());
		}
	}

	let mut file = std::fs::File::create(&pkg_path).map_err(|e| eprintln!("Error: failed to open {} for writing: {}", pkg_path.display(), e))?;
	eprintln!("Downloading {}", package.filename);
	let data = download_file(&pkg_url).await.map_err(|e| eprintln!("Error: {}", e))?;
	file.write_all(&data)
		.map_err(|e| eprintln!("Error: failed to write to {}: {}", pkg_path.display(), e))?;
	Ok(())
}

fn package_url(repository: &Repository, package: &DatabasePackage) -> reqwest::Url {
	let db_path = repository.db_url.path();
	let parent = rpartition(db_path, '/').map(|(parent, _db_name)| parent).unwrap_or("");

	let mut pkg_url = repository.db_url.clone();
	pkg_url.set_path(&format!("{}/{}", parent, package.filename));
	pkg_url
}

/// Get metadata for a file path.
///
/// Returns Ok(None) if the path does not exist.
fn stat(path: impl AsRef<Path>) -> Result<Option<std::fs::Metadata>, ()> {
	let path = path.as_ref();
	match path.metadata() {
		Ok(x) => Ok(Some(x)),
		Err(e) => {
			if e.kind() == std::io::ErrorKind::NotFound {
				Ok(None)
			} else {
				eprintln!("Error: failed to stat {}: {}", path.display(), e);
				Err(())
			}
		},
	}
}
