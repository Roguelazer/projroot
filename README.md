This is a tiny application for finding the root of a project.

[![CI](https://github.com/Roguelazer/projroot/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Roguelazer/projroot/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/projroot)](https://crates.io/crates/projroot)

I would recommend setting it up as an alias in your shell; something like

```fish
alias cdpr "cd (projroot)"
```

This project is heavily-inspired by [vim-rooter](https://github.com/vim-scripts/vim-rooter/tree/master).

If you work a lot with submodules or other use cases where the first VCS directory doesn't represent
the root of your project, you might try the `-m farthest` mode to instead look for the most distant VCS directory.

We currently use the presence of any of the following to determine if something is a project root:

 - `.git`
 - `_darcs`
 - `.hg`
 - `.bzr`
 - `.svn`

This should work on all supported Rust platforms, although the single-filesystem functionality only works
on Unix-like platforms. It even works on WASI with [wasmtime](https://wasmtime.dev/), presuming you grant FS access! 

This work is licensed under the ISC license, a copy of which can be found at [LICENSE.txt](LICENSE.txt)
