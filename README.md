# Coors Lib

Symmetric coroutine library in Rust

```toml
[dependencies.coorslib]
git = "https://github.com/artemvm/coorslib.git"
```

## Usage

Basic usage of symmetric coroutine

```rust
extern crate coorslib;

use coorslib::symmetric::*;

fn main() {
    let mut coors = Coors::new();
    let mut coroutines = Vec::new();

    let coro_1 = Coroutine::spawn(|arg| {
        println!("{}", arg.unwrap());
        println!("{}", coors.yield_to(NEXT, "yay").unwrap());
        coors.yield_to(NEXT, "back");
    });

    let coro_2 = Coroutine::spawn(|arg| {
        println!("{}", arg.unwrap());
        println!("{}", coors.yield_to(NEXT, "mlem").unwrap());
        coors.stop("returning");
    });

    coroutines.push(coro_1);
    coroutines.push(coro_2);
    coors.set_coroutines(coroutines);
    println!("{}", coors.start(FIRST, "starting").unwrap());
}
```

This program will print the following to the console

```
starting
yay
mlem
back
returning
```

# Installation

* Download Coorslib
    - $ git clone https://github.com/artemvm/coorslib.git 

* Install multirust (for nightly)
    - $ curl -sf https://raw.githubusercontent.com/brson/multirust/master/blastoff.sh | sh
    
* Switch to nightly 2015-09-29
    - make sure you are in the directory of Coors Lib (should see Cargo.toml)
    - $ multirust override nightly-2015-09-29
    
* Run tests
    - make sure you are in the directory of Coors Lib
    - $ cargo test

# Uninstallation

* Uninstall multirust
    - $ curl -sf https://raw.githubusercontent.com/brson/multirust/master/blastoff.sh | sh -s -- --uninstall

# Notes

* Currently this crate **can only** be built with Rust nightly because of some unstable features.

* Basically it supports arm, i686, mips, mipsel and x86_64 platforms, but we have only tested in
    - OS X 10.10.*, x86_64, nightly
    - ArchLinux, x86_64, nightly
