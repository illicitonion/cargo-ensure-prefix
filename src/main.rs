#![deny(clippy::all)]
#![deny(clippy::pedantic)]

use cargo::core::Workspace;
use cargo::ops::Packages;
use std::env::current_dir;
use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "cargo-ensure-prefix",
    about = "Ensures the main file for all crates in a workspace have a particular prefix."
)]
struct Opt {
    // Ignored to allow ensure-prefix to optionally be a hidden subcommand as specified by cargo.
    #[structopt(hidden = true)]
    _dummy: Option<String>,

    #[structopt(long, parse(from_os_str))]
    manifest_path: PathBuf,

    #[structopt(short, long)]
    package: Vec<String>,

    #[structopt(long)]
    exclude: Vec<String>,

    #[structopt(long)]
    all: bool,

    #[structopt(long, parse(from_os_str))]
    prefix_path: PathBuf,
}

fn main() {
    //        .version(crate_version!())
    let opt = Opt::from_args();

    let Params {
        paths_to_check,
        prefix,
    } = parse(opt).unwrap_or_else(|err| {
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

fn parse(opt: Opt) -> Result<Params, String> {
    let Opt {
        prefix_path,
        manifest_path,
        all,
        exclude,
        package,
        ..
    } = opt;

    let prefix = std::fs::read_to_string(&prefix_path)
        .map_err(|_| format!("Error reading prefix-path file {}", prefix_path.display()))?;

    let packages = Packages::from_flags(all, exclude, package)
        .map_err(|err| format!("Error parsing package spec: {}", err))?;

    let paths_to_check = list_paths(manifest_path.clone(), &packages)?;

    Ok(Params {
        paths_to_check,
        prefix,
    })
}

fn list_paths(manifest_path: PathBuf, packages: &Packages) -> Result<Vec<PathBuf>, String> {
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

    Ok(packages
        .get_packages(&workspace)
        .map_err(|err| format!("{}", err))?
        .into_iter()
        .flat_map(|package| package.targets())
        .map(|target| target.src_path().path().to_owned())
        .collect())
}

#[cfg(test)]
mod test_list_paths {
    use cargo::core::Workspace;
    use cargo::ops::Packages;
    use cargo::Config;
    use std::collections::HashSet;
    use std::env::current_dir;
    use std::path::PathBuf;

    #[test]
    fn single_package_multiple_explicit_targets() {
        assert_packages(
            &Packages::Default,
            "tests/projects/single_package_explicit_targets/Cargo.toml",
            &["single_package_explicit_targets"],
        );
    }

    #[test]
    fn workspace_default() {
        assert_packages(
            &Packages::Default,
            "tests/projects/workspace_root/Cargo.toml",
            &["workspace_root", "wlib"],
        );
    }

    #[test]
    fn workspace_all() {
        assert_packages(
            &Packages::All,
            "tests/projects/workspace_root/Cargo.toml",
            &["workspace_root", "wbin", "wlib"],
        );
    }

    #[test]
    fn workspace_package_list() {
        assert_packages(
            &Packages::Packages(vec!["wbin".to_owned()].into_iter().collect()),
            "tests/projects/workspace_root/Cargo.toml",
            &["wbin"],
        );
        assert_packages(
            &Packages::Packages(
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
        assert_eq!(
            packages(
                &Packages::Packages(vec!["doesnotexist".to_owned()].into_iter().collect()),
                "tests/projects/workspace_root/Cargo.toml"
            ),
            Err("package `doesnotexist` is not a member of the workspace".to_owned())
        );
    }

    #[test]
    fn manifest_is_in_workspace() {
        assert_packages(
            &Packages::Default,
            "tests/projects/workspace_root/wbin/Cargo.toml",
            &["workspace_root", "wlib"],
        );
        assert_packages(
            &Packages::All,
            "tests/projects/workspace_root/wbin/Cargo.toml",
            &["workspace_root", "wbin", "wlib"],
        );
        assert_packages(
            &Packages::Packages(vec!["wbin".to_owned()].into_iter().collect()),
            "tests/projects/workspace_root/Cargo.toml",
            &["wbin"],
        );
    }

    fn packages(spec: &Packages, manifest_path: &str) -> Result<HashSet<String>, String> {
        let config = Config::default().unwrap();
        let workspace = Workspace::new(&abs(manifest_path), &config).unwrap();
        Ok(spec
            .get_packages(&workspace)
            .map_err(|e| format!("{}", e))?
            .into_iter()
            .map(|p| p.name().as_str().to_owned())
            .collect())
    }

    fn assert_packages(spec: &Packages, manifest_path: &str, expected_packages: &[&str]) {
        assert_eq!(
            packages(spec, manifest_path).unwrap(),
            expected_packages.iter().map(|s| s.to_string()).collect()
        );
    }

    fn abs(s: &str) -> PathBuf {
        current_dir().unwrap().join(s)
    }
}
