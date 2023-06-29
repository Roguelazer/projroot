use anyhow::Context;
use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};

#[cfg(unix)]
mod ancestors_same_filesystem;

#[derive(ValueEnum, Debug, PartialEq, Eq, Clone, Copy)]
enum Mode {
    Closest,
    Farthest,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[cfg(unix)]
    #[clap(
        short,
        long,
        help = "Allow the search to traverse filesystems (otherwise, stops at FS boundary)"
    )]
    span_file_systems: bool,
    #[clap(
        short,
        long,
        help = "Start the search in the given directory (defaults to the cwd)"
    )]
    workdir: Option<PathBuf>,
    #[clap(short, long, value_enum, default_value_t = Mode::Closest)]
    mode: Mode,
}

const INDICATORS: &[&str] = &[".git", "_darcs", ".hg", ".bzr", ".svn"];

fn is_project_root<P: AsRef<Path>>(dir: &P) -> bool {
    let p = dir.as_ref();

    INDICATORS.iter().any(|i| p.join(i).exists())
}

#[inline(always)]
#[allow(unused_variables)]
fn ancestors(
    starting_directory: &Path,
    span_file_systems: bool,
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<&Path>>> {
    cfg_if::cfg_if! {
        if #[cfg(unix)] {
            ancestors_same_filesystem::Ancestors::new(starting_directory, starting_directory.ancestors(), span_file_systems)
        } else {
            Ok(starting_directory.ancestors().map(|i| Ok(i)))
        }
    }
}

fn find_project_root(
    starting_directory: &Path,
    span_file_systems: bool,
    mode: Mode,
) -> anyhow::Result<Option<PathBuf>> {
    let mut last_candidate: Option<PathBuf> = None;

    for path in ancestors(starting_directory, span_file_systems)? {
        let path = path?;
        if is_project_root(&path) {
            if mode == Mode::Closest {
                return Ok(Some(path.to_path_buf()));
            } else {
                last_candidate.replace(path.to_owned());
            }
        }
    }
    if let Some(path) = last_candidate {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let starting_directory = args
        .workdir
        .unwrap_or(std::env::current_dir().context("could not determine cwd")?);

    #[cfg(not(target_arch = "wasm32"))]
    let starting_directory =
        std::fs::canonicalize(starting_directory).context("could not canonicalize path")?;

    #[cfg(unix)]
    let span_file_systems = args.span_file_systems;
    #[cfg(not(unix))]
    let span_file_systems = true;

    if let Some(path) = find_project_root(&starting_directory, span_file_systems, args.mode)? {
        println!("{}", path.as_os_str().to_string_lossy());
        Ok(())
    } else {
        anyhow::bail!(
            "found no project root in ancestors of {}",
            starting_directory.as_os_str().to_string_lossy()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{find_project_root, is_project_root, Mode};

    #[test]
    fn test_is_project_root_git() -> anyhow::Result<()> {
        let t = tempfile::tempdir()?;
        assert!(!is_project_root(&t.path()));
        std::fs::create_dir(t.path().join(".git"))?;
        assert!(is_project_root(&t.path()));
        Ok(())
    }

    #[test]
    fn test_is_project_root_svn() -> anyhow::Result<()> {
        let t = tempfile::tempdir()?;
        assert!(!is_project_root(&t.path()));
        std::fs::create_dir(t.path().join(".svn"))?;
        assert!(is_project_root(&t.path()));
        Ok(())
    }

    #[test]
    fn test_find_project_root_mode() -> anyhow::Result<()> {
        let t = tempfile::tempdir()?;
        std::fs::create_dir(t.path().join(".git"))?;
        std::fs::create_dir(t.path().join("foo"))?;
        std::fs::create_dir(t.path().join("foo").join("bar"))?;
        std::fs::create_dir(t.path().join("foo").join("bar").join(".git"))?;

        let closest = find_project_root(&t.path().join("foo").join("bar"), false, Mode::Closest)?;
        assert_eq!(closest, Some(t.path().join("foo").join("bar")));

        let farthest = find_project_root(&t.path().join("foo").join("bar"), false, Mode::Farthest)?;
        assert_eq!(farthest, Some(t.path().to_owned()));
        Ok(())
    }
}
