# real_parent

Provides a path extension method `real_parent` which is safe in the presence of symlinks.

Noting that `Path::parent` gives incorrect results in the presence of symlinks, `Path::canonicalize` has been used extensively to mitigate this.
This comes, however, with some ergonomic drawbacks (see below).

The goal of this crate is to replace eager and early calls to `Path::canonicalize` with late calls to `PathExt::real_parent`.

In this way, the user's preferred and natural view of their filesystem is preserved, and parent paths are resolved correctly on a just-in-time basis.

The extensive [tests](https://github.com/tesujimath/real_parent/blob/main/tests/path_ext.rs) are the best documentation of behaviour for various edge cases.

## Rationale

The standard library method `Path::parent` is not safe in the presence of symlinks.
For background information and a comprehensive analysis see the Plan 9 paper [Getting Dot-Dot Right](https://9p.io/sys/doc/lexnames.html).

So far the Rust community has leaned extensively on `Path::canonicalize` to mitigate this.  While this approach avoids path breakage, it has
the unpleasant result of making all paths absolute, and resolving symlinks into their underlying physical paths.

Considering that symlinks exist in part to provide an abstracted view of the filesystem, eagerly resolving symlinks to their physical paths
could be seen as a violation of encapsulation.  That is, a user may prefer to deal with their filesystem in terms of their symlinks rather than being
exposed to absolute physical paths.  There is a further gain in preferring relative paths to absolute, namely independence from absolute location
in the filesystem.

Two scenarios illustate this problem.

### Nix

Nix, and expecially Nix Home Manager, make heavy use of symlinks into the Nix store, which is where all software and its configuration is installed.

For example:

```text
aya> ls -l ~/.config/nushell/*.nu | select name type target
╭───┬─────────────────────────────────────┬─────────┬──────────────────────────────────────────────────────────────────────────────────────────╮
│ # │                name                 │  type   │                                          target                                          │
├───┼─────────────────────────────────────┼─────────┼──────────────────────────────────────────────────────────────────────────────────────────┤
│ 0 │ /home/sjg/.config/nushell/config.nu │ symlink │ /nix/store/vjx9vvdq2nqiz0dmqly9la8mqyz2lcnr-home-manager-files/.config/nushell/config.nu │
│ 1 │ /home/sjg/.config/nushell/env.nu    │ symlink │ /nix/store/vjx9vvdq2nqiz0dmqly9la8mqyz2lcnr-home-manager-files/.config/nushell/env.nu    │
│ 2 │ /home/sjg/.config/nushell/plugin.nu │ file    │                                                                                          │
╰───┴─────────────────────────────────────┴─────────┴──────────────────────────────────────────────────────────────────────────────────────────╯
```

It would be more ergonomic to avoid surfacing these underlying Nix store paths to the user quite so eagerly.

### GNU Stow

[GNU Stow](https://www.gnu.org/software/stow/manual/html_node/index.html) is a symlink farm manager.  It is often used for managing user dotfiles, or `/usr/local`.

Use of GNU Stow results in extensive symlink farms, with files appearing to exist in well-known directories alongside one another, where in reality they are symlinks to various locations in the filesystem.

## Supported Platforms

`real_parent` runs on all platforms, with the following caveats on Windows.

- since the tests create symbolic links, to run the tests on Windows you need to run as administrator 🤯

- symbolic link behaviour on Windows is awkward, so some tests have had to be disabled on that platform

Isolating exactly what is the cause for weird failures with symbolic link edge cases on Windows is beyond both this author's level of Windows platform expertise and, frankly, interest.  Pull requests welcome in this area.  Note however that the standard library `Path::canonicalize` may also fail in these edge cases.


## License

Licensed under either of

 * Apache License, Version 2.0
   [LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   [LICENSE-MIT](http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
