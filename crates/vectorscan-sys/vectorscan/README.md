# About Vectorscan

A fork of Intel's Hyperscan, modified to run on more platforms. Currently ARM NEON/ASIMD
is 100% functional, and Power VSX are in development. ARM SVE2 support is in ongoing with
access to hardware now. More platforms will follow in the future.

Vectorscan will follow Intel's API and internal algorithms where possible, but will not
hesitate to make code changes where it is thought of giving better performance or better
portability. In addition, the code will be gradually simplified and made more uniform and
all architecture specific -currently Intel- #ifdefs will be removed and abstracted away.

# Why was there a need for a fork?

Originally, the ARM porting was intended to be merged into Intel's own Hyperscan, and relevant 
Pull Requests were made to the project for this reason. Unfortunately, the
PRs were rejected for now and the forseeable future, thus we have created Vectorscan for 
our own multi-architectural and opensource collaborative needs.

The recent license change of Hyperscan makes Vectorscan even more relevant for the FLOSS ecosystem.

# What is Vectorscan/Hyperscan/?

Hyperscan and by extension Vectorscan is a high-performance multiple regex matching library. It follows the
regular expression syntax of the commonly-used libpcre library, but is a
standalone library with its own C API.

Hyperscan/Vectorscan uses hybrid automata techniques to allow simultaneous matching of
large numbers (up to tens of thousands) of regular expressions and for the
matching of regular expressions across streams of data.

Vectorscan is typically used in a DPI library stack, just like Hyperscan.

# License

Vectorscan follows a BSD License like the original Hyperscan (up to 5.4).

Vectorscan continues to be an open source project and we are committed to keep it that way.
See the LICENSE file in the project repository.

## Hyperscan License Change after 5.4

According to
[Accelerate Snort Performance with Hyperscan and Intel Xeon Processors on Public Clouds](https://networkbuilders.intel.com/docs/networkbuilders/accelerate-snort-performance-with-hyperscan-and-intel-xeon-processors-on-public-clouds-1680176363.pdf) versions of Hyperscan later than 5.4 are
going to be closed-source:

> The latest open-source version (BSD-3 license) of Hyperscan on Github is 5.4. Intel conducts continuous internal
> development and delivers new Hyperscan releases under Intel Proprietary License (IPL) beginning from 5.5 for interested
> customers. Please contact authors to learn more about getting new Hyperscan releases.

# Versioning

The `master` branch on Github will always contain the most recent stable release of
Hyperscan. Each version released to `master` goes through QA and testing before
it is released; if you're a user, rather than a developer, this is the version
you should be using.

Further development towards the next release takes place on the `develop`
branch. All PRs are first made against the develop branch and if the pass the [Vectorscan CI](https://buildbot-ci.vectorcamp.gr/#/grid), then they get merged. Similarly with PRs from develop to master.

# Compatibility with Hyperscan

Vectorscan aims to be ABI and API compatible with the last open source version of Intel Hyperscan 5.4.
After careful consideration we decided that we will **NOT** aim to achieving compatibility with later Hyperscan versions 5.5/5.6 that have extended Hyperscan's API.
If keeping up to date with latest API of Hyperscan, you should talk to Intel and get a license to use that.
However, we intend to extend Vectorscan's API with user requested changes or API extensions and improvements that we think are best for the project.

# Installation

## Debian/Ubuntu

On recent Debian/Ubuntu systems, vectorscan should be directly available for installation:

```
$ sudo apt install libvectorscan5
```

Or to install the devel package you can install `libvectorscan-dev` package:

```
$ sudo apt install libvectorscan-dev
```

For other distributions/OSes please check the [Wiki](https://github.com/VectorCamp/vectorscan/wiki/Installation-from-package)


# Build Instructions

The build system has recently been refactored to be more modular and easier to extend. For that reason,
some small but necessary changes were made that might break compatibility with how Hyperscan was built.

## Install Common Dependencies

### Debian/Ubuntu
In order to build on Debian/Ubuntu make sure you install the following build-dependencies

```
$ sudo apt build-essential cmake ragel pkg-config libsqlite3-dev libpcap-dev
```

### Other distributions

TBD

### MacOS X (M1/M2/M3 CPUs only)

Assuming an existing HomeBrew installation:

```
% brew install boost cmake gcc libpcap pkg-config ragel sqlite
```

## Configure & build

In order to configure with `cmake` first create and cd into a build directory:

```
$ mkdir build
$ cd build
```

Then call `cmake` from inside the `build` directory:

```
$ cmake ../
```

Common options for Cmake are:

* `-DBUILD_STATIC_LIBS=[On|Off]` Build static libraries
* `-DBUILD_SHARED_LIBS=[On|Off]` Build shared libraries (if none are set static libraries are built by default)
* `-DCMAKE_BUILD_TYPE=[Release|Debug|RelWithDebInfo|MinSizeRel]` Configure build type and determine optimizations and certain features.
* `-DUSE_CPU_NATIVE=[On|Off]` Native CPU detection is off by default, however it is possible to build a performance-oriented non-fat library tuned to your CPU
* `-DFAT_RUNTIME=[On|Off]` Fat Runtime is only available for X86 32-bit/64-bit and AArch64 architectures and only on Linux. It is incompatible with `Debug` type and `USE_CPU_NATIVE`.

### Specific options for X86 32-bit/64-bit (Intel/AMD) CPUs

* `-DBUILD_AVX2=[On|Off]` Enable code for AVX2.
* `-DBUILD_AVX512=[On|Off]` Enable code for AVX512. Implies `BUILD_AVX2`.
* `-DBUILD_AVX512VBMI=[On|Off]` Enable code for AVX512 with VBMI extension. Implies `BUILD_AVX512`.

### Specific options for Arm 64-bit CPUs

* `-DBUILD_SVE=[On|Off]` Enable code for SVE, like on AWS Graviton3 CPUs. Not much code is ported just for SVE , but enabling SVE code production, does improve code generation, see [Benchmarks](https://github.com/VectorCamp/vectorscan/wiki/Benchmarks).
* `-DBUILD_SVE2=[On|Off]` Enable code for SVE2, implies `BUILD_SVE`. Most non-Neon code is written for SVE2
* `-DBUILD_SVE2_BITPERM=[On|Off]` Enable code for SVE2_BITPERM harwdare feature, implies `BUILD_SVE2`.

## Other options

* `SANITIZE=[address|memory|undefined]` (experimental) Use `libasan` sanitizer to detect possible bugs. For now only `address` is tested. This will eventually be integrated in the CI.

## Build

If `cmake` has completed successfully you can run `make` in the same directory, if you have a multi-core system with `N` cores, running

```
$ make -j <N>
```

will speed up the process. If all goes well, you should have the vectorscan library compiled.


# Contributions

The official homepage for Vectorscan is at [www.github.com/VectorCamp/vectorscan](https://www.github.com/VectorCamp/vectorscan).

# Vectorscan Development

All development of Vectorscan is done in public. 

# Original Hyperscan links
For reference, the official homepage for Hyperscan is at [www.hyperscan.io](https://www.hyperscan.io).

# Hyperscan Documentation

Information on building the Hyperscan library and using its API is available in
the [Developer Reference Guide](http://intel.github.io/hyperscan/dev-reference/).

And you can find the source code [on Github](https://github.com/intel/hyperscan).

For Intel Hyperscan related issues and questions, please follow the relevant links there.