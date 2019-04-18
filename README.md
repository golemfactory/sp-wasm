# SpiderMonkey-based WebAssembly Sandbox
[![Build Status]][travis] [![Rustc 1.33]][rustc] [![License]][license]

[Build Status]: https://travis-ci.org/golemfactory/sp-wasm.svg?branch=master
[travis]: http://travis-ci.org/golemfactory/sp-wasm
[Rustc 1.33]: https://img.shields.io/badge/rustc-1.33+-lightgray.svg
[rustc]: https://blog.rust-lang.org/2019/02/28/Rust-1.33.0.html
[License]: https://img.shields.io/github/license/golemfactory/sp-wasm.svg 
[license]: https://www.gnu.org/licenses/gpl-3.0.en.html

A WebAssembly sandbox using standalone SpiderMonkey engine. For `v8` version,
see [golemfactory/v8-wasm](https://github.com/golemfactory/v8-wasm).

This WebAssembly sandbox is used in current development version of
Golem: [golem/apps/wasm](https://github.com/golemfactory/golem/tree/develop/apps/wasm).
If you would like to launch a Wasm task in Golem, see
[here](https://docs.golem.network/#/About/Use-Cases?id=wasm).

- [SpiderMonkey-based WebAssembly Sandbox](#spidermonkey-based-webassembly-sandbox)
  - [Quick start guide](#quick-start-guide)
    - [1. Create and cross-compile simple program](#1-create-and-cross-compile-simple-program)
      - [1.1 C/C++](#11-cc)
      - [1.2 Rust](#12-rust)
    - [2. Create input and output dirs and files](#2-create-input-and-output-dirs-and-files)
    - [3. Run!](#3-run)
  - [Build instructions](#build-instructions)
    - [Using Docker (recommended)](#using-docker-recommended)
    - [Natively on Linux](#natively-on-linux)
    - [Natively on other OSes](#natively-on-other-oses)
  - [CLI arguments explained](#cli-arguments-explained)
  - [Caveats](#caveats)
  - [Wasm store](#wasm-store)
  - [Contributing](#contributing)
  - [License](#license)

## Quick start guide
This guide assumes you have successfully built the `wasm-sandbox` binary; for build instructions, see section
[Build instructions](#build-instructions) below. If you are running Linux, then you can also use the prebuilt
binaries from [here](https://github.com/golemfactory/sp-wasm/releases).

### 1. Create and cross-compile simple program
Let us create a simple `hello world` style program which will read in
some text from `in.txt` text file, read your name from the command line,
and save the resultant text in `out.txt`. We'll demonstrate how to
cross-compile apps to Wasm for use in Golem in two languages of choice:
C and Rust.

#### 1.1 C/C++
```c
#include <stdio.h>

int main(int argc, char** argv) {
  char* name = argc >= 2 ? argv[1] : "anonymous";
  size_t len = 0;
  char* line = NULL;
  ssize_t read;
  
  FILE* f_in = fopen("in.txt", "r");
  FILE* f_out = fopen("out.txt", "w");
  
  while ((read = getline(&line, &len, f_in)) != -1)
      fprintf(f_out, "%s\n", line);
  
  fprintf(f_out, "%s\n", name);
  
  fclose(f_out);
  fclose(f_in);
  
  return 0;
}
```

There is one important thing to notice here. The sandbox communicates
the results of computation by reading and writing to files. Thus, every
Wasm program is required to at the very least create an output file.
If your code does not include file manipulation in its main body,
then the Emscripten compiler, by default, will not initialise
JavaScript `FS` library, and will trip the sandbox. This will also be true
for programs cross-compiled [from Rust](#12-rust).

Now, we can try and compile the program with Emscripten.
In order to do that you need Emscripten SDK installed on your
system. For instructions on how to do it, see
[here](https://emscripten.org/docs/getting_started/downloads.html).

```
$ emcc -o simple.js -s BINARYEN_ASYNC_COMPILATION=0 simple.c
```

Emscripten will then produce two files: `simple.js` and `simple.wasm`.
The produced JavaScript file acts as glue code and sets up all of
the rudimentary syscalls in JavaScript such as `MemFS` (in-memory
filesystem), etc., while the `simple.wasm` is our C program
cross-compiled to Wasm.

Note here the compiler flag `-s BINARYEN_ASYNC_COMPILATION=0`. By
default, the Emscripten compiler enables async IO lib when
cross-compiling to Wasm which we currently do not support.
Therefore, in order to alleviate the problem, make sure to always
cross-compile with `-s BINARYEN_ASYNC_COMPILATION=0` flag.

#### 1.2 Rust
With Rust, firstly go ahead and create a new binary with `cargo`
```
$ cargo new --bin simple
```

Then go ahead and paste the following to `simple/src/main.rs`
file
```rust
use std::env;
use std::fs;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let name = args.get(1).map_or("anonymous".to_owned(), |x| x.clone());

    let mut in_file = fs::File::open("in.txt")?;
    let mut contents = String::new();
    in_file.read_to_string(&mut contents)?;

    let mut out_file = fs::File::create("out.txt")?;
    out_file.write_all(&contents.as_bytes())?;
    out_file.write_all(&name.as_bytes())?;

    Ok(())
}
```

As was the case with [C program](#11-c/c++), it is important to notice
here that the sandbox communicates
the results of computation by reading and writing to files. Thus, every
Wasm program is required to at the very least create an output file.
If your code does not include file manipulation in its main body,
then the Emscripten compiler, by default, will not initialise
JavaScript `FS` library, and will trip the sandbox.

In order to cross-compile Rust to Wasm compatible with Golem's
sandbox, firstly we need to install the required target which
is `wasm32-unknown-emscripten`. The easiest way of doing so, as well
as generally managing your Rust installations, is to use
[rustup](https://rustup.rs/)
```
$ rustup target add wasm32-unknown-emscripten
```

Note that cross-compiling Rust to this target still requires that you
have Emscripten SDK installed on your
system. For instructions on how to do it, see
[here](https://emscripten.org/docs/getting_started/downloads.html).

Now, we can compile our Rust program to Wasm. Make sure you are in 
the root of your Rust crate, i.e., at the top of `simple`
if you didn't change the name of your crate, and run
```
$ cargo rustc --target=wasm32-unknown-emscripten --release -- \
  -C link-args="-s BINARYEN_ASYNC_COMPILATION=0"
```

If everything went OK, you should now see two files:
`simple.js` and `simple.wasm` in `simple/target/wasm32-unknown-emscripten/release`.
Just like in [C program](#11-cc++)'s case, the produced JavaScript
file acts as glue code and sets up all of
the rudimentary syscalls in JavaScript such as `MemFS` (in-memory
filesystem), etc., while the `simple.wasm` is our Rust program
cross-compiled to Wasm.

Again, note here the compiler flag `-s BINARYEN_ASYNC_COMPILATION=0` passed as
additional compiler flags to `rustc`. By
default, when building for target `wasm32-unknown-emscripten` with `rustc`
the compiler will cross-compile with default Emscripten compiler flags which
require async IO lib when cross-compiling to Wasm which we currently
do not support. Therefore, in order to alleviate the problem, make
sure to always cross-compile with `-s BINARYEN_ASYNC_COMPILATION=0` flag.

### 2. Create input and output dirs and files
The sandbox will require us to specify input and output paths together
with output filenames to create, and any additional arguments (see
[CLI arguments explained](#cli-arguments-explained) section below
for detailed specification
of the required arguments). Suppose we have the following file
structure locally

```
  |-- in/
  |    |
  |    |-- in.txt
  |
  |-- out/
```

Paste the following text in the `in.txt` file

```
// in.txt
You are running Wasm!
```

### 3. Run!
After you have successfully run all of the above steps up to now, you should have the following file structure locally

```
  |-- simple.js
  |-- simple.wasm
  |
  |-- in/
  |    |
  |    |-- in.txt
  |
  |-- out/
```

We can now run our Wasm binary inside the sandbox

1. using Docker (if you've followed [Using Docker (recommended)](#using-docker-recommended)
   build instructions)

```
docker run --mount type=bind,source=$PWD,target=/workdir --workdir /workdir \
            wasm-sandbox:latest -I in/ -O out/ -j simple.js -w simple.wasm \
            -o out.txt -- "<your_name>"
```

2. natively (if you're using the prebuilt binaries, or you've built natively following
   [Natively on Linux](#natively-on-linux) build instructions)

```
$ wasm-sandbox -I in/ -O out/ -j simple.js -w simple.wasm \
               -o out.txt -- "<your_name>"
```

Here, `-I` maps the input dir with *all* its contents (files and
subdirs) directly to the root `/` in `MemFS`. The output files,
on the other hand, will be saved in `out/` local dir. The names of
the expected output files have to match those specified with `-o`
flags. Thus, in this case, our Wasm bin is expected to create an
output file `/out.txt` in `MemFS` which will then be saved in
`out/out.txt` locally.

After you execute Wasm bin in the sandbox, `out.txt` should be
created in `out/` dir

```
  |-- simple.js
  |-- simple.wasm
  |
  |-- in/
  |    |
  |    |-- in.txt
  |
  |-- out/
  |    |
  |    |-- out.txt
```

with the contents similar to the following

```
// out.txt
You are running Wasm!
<your_name>
```

## Build instructions
### Using Docker (recommended)
To build using Docker, simply run

```
$ ./build.sh
```

If you are running Windows, then you can invoke the command in the shell script manually
in the command line as follows

```
docker build -t wasm-sandbox:latest .
```

### Natively on Linux
To build natively on Linux, you need to follow the installation instructions of
[servo/rust-mozjs](https://github.com/servo/rust-mozjs) and
[servo/mozjs](https://github.com/servo/mozjs). The latter is Mozilla's Servo's SpiderMonkey fork and low-level
Rust bindings, and as such, requires C/C++ compiler and Autoconf 2.13. See [servo/mozjs/README.md](https://github.com/servo/mozjs)
for detailed building instructions.

After following the aforementioned instructions, to build the sandbox, run

```
$ cargo build
```

If you would like to build with SpiderMonkey's debug symbols and extensive logging, run instead

```
$ cargo build --features "debugmozjs"
```

### Natively on other OSes
We currently do not offer any support for building the sandbox natively on other OSes.

## CLI arguments explained
```
wasm-sandbox -I <input-dir> -O <output-dir> -j <wasm-js> -w <wasm> -o <output-file>... -- <args>...
```

where
* `-I` path to the input dir
* `-O` path to the output dir
* `-j` path to the Emscripten JS glue script
* `-w` path to the Emscripten WASM binary
* `-o` paths to expected output files
* `--` anything after this will be passed to the WASM binary as arguments

By default, basic logging is enabled. If you would like to enable more comprehensive logging, export
the following variable

```
RUST_LOG=debug
```

## Caveats
* If you were following the [Quick start guide](#quick-start-guide)
  you already know that
  every Wasm bin needs to be cross-compiled by Emscripten with
  `-s BINARYEN_ASYNC_COMPILATION=0` flag in order to turn off the use
  of async IO which we currently don't support.
* Sometimes, if the binary you are cross-compiling is of substantial
  size, you might encounter a `asm2wasm` validation error stating
  that there is not enough memory assigned to Wasm. In this case,
  you can circumvent the problem by adding `-s TOTAL_MEMORY=value`
  flag. The value has to be an integer multiple of 1 Wasm memory page
  which is currently set at `65,536` bytes.
* When running your Wasm binary you encounter an `OOM` error at
  runtime, it usually means that the sandbox has run out-of-memory.
  To alleviate the problem, recompile your program with
  `-s ALLOW_MEMORY_GROWTH=1`.
* Emscripten, by default, doesn't support `/dev/(u)random` emulation
  targets different than either browser or `nodejs`. Therefore, we
  have added basic emulation of the random device that is *fully*
  deterministic. For details, see [#5](https://github.com/golemfactory/sp-wasm/pull/5).

## Wasm store
More examples of precompiled Wasm binaries can be found in [golemfactory/wasm-store](https://github.com/golemfactory/wasm-store) repo.

## Contributing
All contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for pointers.

## License
Licensed under [GNU General Public License v3.0](LICENSE).
