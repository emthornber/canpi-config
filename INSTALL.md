# CANPi Configuration Library

The functionality is distributed as a Rust crate that is accessed from GitHub using a tag or a branch (or the latest on the default branch).
e.g.
```
[dependencies]
canpi-config = { git = "https://github.com/emthornber/canpi-config.git", tag = "v0.1.4" }
```
or
```
[dependencies]
canpi-config = { git = "https://github.com/emthornber/canpi-config.git", branch = "lgtrunk" }
```
or
```
[dependencies]
canpi-config = { git = "https://github.com/emthornber/canpi-config.git" }
```

## Compiling

The source code will be compiled as part of the build of the calling executable along with all the other dependant crates.

