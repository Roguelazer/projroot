use std::fs::metadata;
use std::marker::PhantomData;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use anyhow::Context;

pub(crate) struct Ancestors<'a, I> {
    inner: I,
    devno: u64,
    span_file_system: bool,
    _phantom: PhantomData<&'a I>,
}

impl<'a, I> Ancestors<'a, I>
where
    I: Iterator<Item = &'a Path>,
{
    pub fn new(p: &Path, iterator: I, span_file_system: bool) -> anyhow::Result<Self> {
        let devno = metadata(p)
            .context("could not stat initial directory")?
            .dev();
        Ok(Self {
            inner: iterator,
            devno,
            span_file_system,
            _phantom: PhantomData,
        })
    }
}

impl<'a, I: Iterator<Item = &'a Path>> Iterator for Ancestors<'a, I> {
    type Item = anyhow::Result<&'a Path>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.next()?;
        if !self.span_file_system {
            let devno = match metadata(next).context("could not stat ancestor") {
                Ok(m) => m,
                Err(e) => return Some(Err(e)),
            }
            .dev();
            if devno != self.devno {
                return Some(Err(anyhow::anyhow!(
                    "traversed filesystems without finding project root"
                )));
            }
        }
        Some(Ok(next))
    }
}
