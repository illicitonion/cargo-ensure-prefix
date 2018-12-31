#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use cargo::core::Members;
use cargo::core::Package;
use cargo::core::Workspace;
use clap::{crate_version, App, Arg, ArgMatches};
use std::collections::HashSet;
use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let matches = App::new("cargo-ensure-prefix")
        .version(crate_version!())
        .arg(
            Arg::with_name("manifest-path")
                .long("manifest-path")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("package")
                .long("package")
                .short("p")
                .takes_value(true)
                .multiple(true),
        )
        .arg(Arg::with_name("all").long("all"))
        .arg(
            Arg::with_name("prefix-path")
                .long("prefix-path")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let Params {
        paths_to_check,
        prefix,
    } = parse(&matches).unwrap_or_else(|err| {
        die(&err);
        unreachable!();
    });

    if paths_to_check.is_empty() {
        die("Didn't find matching package(s)");
    }

    let mut violations = vec![];

    let mut buf = vec![0; prefix.len()];

    for path in &paths_to_check {
        let mut file = std::fs::File::open(path).expect("Error reading source file");
        let has_prefix = match file.read_exact(&mut buf) {
            Ok(()) => prefix
                .bytes()
                .zip(buf.iter())
                .all(|(want, got)| want == *got || want == 0x1A),
            Err(ref err) if err.kind() == std::io::ErrorKind::UnexpectedEof => false,
            Err(ref err) => {
                eprintln!("Error reading {}: {}", path.display(), err);
                false
            }
        };
        if !has_prefix {
            violations.push(path.to_owned());
        }
    }

    violations.sort();

    for violation in &violations {
        println!("{}", violation.display());
    }
    if !violations.is_empty() {
        std::process::exit(1);
    }
}

fn die(message: &str) {
    eprintln!("{}", message);
    std::process::exit(2);
}

struct Params {
    paths_to_check: Vec<PathBuf>,
    prefix: String,
}

fn parse(matches: &ArgMatches) -> Result<Params, String> {
    let prefix_path = matches.value_of("prefix-path").unwrap();
    let prefix = std::fs::read_to_string(prefix_path)
        .map_err(|_| format!("Error reading prefix-path file {}", prefix_path))?;

    let package_filter = match (matches.is_present("all"), matches.values_of("package")) {
        (true, None) => PackageFilter::All,
        (false, None) => PackageFilter::Default,
        (false, Some(packages)) => PackageFilter::Packages(packages.map(str::to_owned).collect()),
        (true, Some(_)) => return Err("Cannot specify --all and --package".to_owned()),
    };

    let paths_to_check = list_paths(
        PathBuf::from(matches.value_of("manifest-path").unwrap()),
        &package_filter,
    )?;

    Ok(Params {
        paths_to_check,
        prefix,
    })
}

#[derive(Debug)]
enum PackageFilter {
    All,
    Default,
    Packages(HashSet<String>),
}

impl PackageFilter {
    fn members<'a>(&'a self, workspace: &'a Workspace) -> Members<'a, 'a> {
        match self {
            PackageFilter::Default => workspace.default_members(),
            PackageFilter::All | PackageFilter::Packages(_) => workspace.members(),
        }
    }

    fn filter(&self, package: &Package) -> bool {
        match self {
            PackageFilter::Packages(ref packages) => packages.contains(package.name().as_str()),
            PackageFilter::All | PackageFilter::Default => true,
        }
    }

    fn packages<'a>(&'a self, workspace: &'a Workspace) -> impl Iterator<Item = &'a Package> {
        self.members(workspace).filter(move |p| self.filter(p))
    }
}

fn list_paths(
    manifest_path: PathBuf,
    package_filter: &PackageFilter,
) -> Result<Vec<PathBuf>, String> {
    let mut manifest_path = manifest_path;
    if !manifest_path.exists() {
        return Err(format!("Could not find {}", manifest_path.display()));
    }
    if !manifest_path.is_absolute() {
        manifest_path = current_dir().unwrap().join(manifest_path);
    }

    let config = cargo::util::config::Config::default()
        .map_err(|err| format!("Error making cargo config: {}", err))?;
    let workspace = Workspace::new(&manifest_path, &config).unwrap_or_else(|_| {
        die(&format!("Error parsing {}", manifest_path.display()));
        unreachable!();
    });

    Ok(package_filter
        .packages(&workspace)
        .flat_map(|package| package.targets())
        .map(|target| target.src_path().path().to_owned())
        .collect())
}

#[cfg(test)]
mod test_list_paths {
    use super::PackageFilter;
    use cargo::core::Workspace;
    use cargo::Config;
    use std::collections::HashSet;
    use std::env::current_dir;
    use std::path::PathBuf;

    #[test]
    fn single_package_multiple_explicit_targets() {
        assert_packages(
            &PackageFilter::Default,
            "tests/projects/single_package_explicit_targets/Cargo.toml",
            &["single_package_explicit_targets"],
        );
    }

    #[test]
    fn workspace_default() {
        assert_packages(
            &PackageFilter::Default,
            "tests/projects/workspace_root/Cargo.toml",
            &["workspace_root", "wlib"],
        );
    }

    #[test]
    fn workspace_all() {
        assert_packages(
            &PackageFilter::All,
            "tests/projects/workspace_root/Cargo.toml",
            &["workspace_root", "wbin", "wlib"],
        );
    }

    #[test]
    fn workspace_package_list() {
        assert_packages(
            &PackageFilter::Packages(vec!["wbin".to_owned()].into_iter().collect()),
            "tests/projects/workspace_root/Cargo.toml",
            &["wbin"],
        );
        assert_packages(
            &PackageFilter::Packages(
                vec!["wbin", "workspace_root"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            ),
            "tests/projects/workspace_root/Cargo.toml",
            &["wbin", "workspace_root"],
        );
    }

    #[test]
    fn workspace_package_not_found() {
        assert_packages(
            &PackageFilter::Packages(vec!["doesnotexist".to_owned()].into_iter().collect()),
            "tests/projects/workspace_root/Cargo.toml",
            &[],
        );
    }

    #[test]
    fn manifest_is_in_workspace() {
        assert_packages(
            &PackageFilter::Default,
            "tests/projects/workspace_root/wbin/Cargo.toml",
            &["workspace_root", "wlib"],
        );
        assert_packages(
            &PackageFilter::All,
            "tests/projects/workspace_root/wbin/Cargo.toml",
            &["workspace_root", "wbin", "wlib"],
        );
        assert_packages(
            &PackageFilter::Packages(vec!["wbin".to_owned()].into_iter().collect()),
            "tests/projects/workspace_root/Cargo.toml",
            &["wbin"],
        );
    }

    fn assert_packages(
        package_filter: &PackageFilter,
        manifest_path: &str,
        expected_packages: &[&str],
    ) {
        let config = Config::default().unwrap();
        let workspace = Workspace::new(&abs(manifest_path), &config).unwrap();
        let got_packages: HashSet<_> = package_filter
            .members(&workspace)
            .filter(|p| package_filter.filter(p))
            .map(|p| p.name().as_str().to_owned())
            .collect();
        assert_eq!(
            got_packages,
            expected_packages.iter().map(|s| s.to_string()).collect()
        );
    }

    fn abs(s: &str) -> PathBuf {
        current_dir().unwrap().join(s)
    }
}
