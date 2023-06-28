use anyhow::Context;
use clap::{Parser, ValueEnum};
#[cfg(unix)]
use std::fs::metadata;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

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

#[cfg(unix)]
#[inline(always)]
fn get_device(p: &Path) -> anyhow::Result<u64> {
    Ok(metadata(p)?.dev())
}

#[cfg(not(unix))]
#[inline(always)]
fn get_device(_p: &Path) -> anyhow::Result<u64> {
    Ok(0)
}

fn find_project_root(
    starting_directory: &Path,
    span_file_systems: bool,
    mode: Mode,
) -> anyhow::Result<Option<PathBuf>> {
    let starting_device =
        get_device(starting_directory).context("could not stat starting directory")?;

    let mut last_candidate: Option<PathBuf> = None;

    for path in starting_directory.ancestors() {
        if !span_file_systems
            && get_device(path).context("could not stat ancestor")? != starting_device
        {
            if let Some(path) = last_candidate {
                return Ok(Some(path));
            } else {
                anyhow::bail!("traversed filesystems without finding project root");
            }
        }
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
