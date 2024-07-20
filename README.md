# yggdrasil 🌲

yggdrasil is the robot framework built by the Dutch Nao Team for the SPL competition.

## Building

yggdrasil supports building for both Linux and macOS. Building on Windows is not supported, but it is possible to build using WSL.

Compilation of yggdrasil is handled by sindri, our tool that simplifies robot interaction by automating the process of building and deploying on the robots. It also has other useful features like logging and scanning the network for robots.

After installing all of the dependencies, install sindri to your system once by running:

```bash
cargo install --locked --path crates/sindri
```

After installing sindri, you can use it from the command line to build and deploy yggdrasil to the robot in one command:

```bash
sindri run <robot-number>
```

To see all available commands, run:

```bash
sindri -h
```

When making changes to sindri, you need to run the following command for the changes to take effect:

```
sindri update
```

## Dependencies

yggdrasil is built using Rust, and as such requires the Rust toolchain to be installed.
We recommend installing it using rustup, which can be found [here](https://rustup.rs/).

The robots run our own Arch linux based distribution, and as such the `x86_64-unknown-linux-gnu` target is required for cross compilation:

```bash
rustup target add x86_64-unknown-linux-gnu
```

### Linux

Building for Linux is incredibly simple, as it only requires the following dependencies:

**Ubuntu**

```bash
sudo apt-get install cmake libasound2-dev libv4l-dev nasm
```

**Arch**

```bash
sudo pacman -S cmake alsa-lib v4l-utils nasm
```

### macOS

Building for macOS is supported for both ARM/Intel macs, but it does require a cross compilation toolchain.

Both the toolchain and the required libraries can be installed using Homebrew:

```bash
# First we add the tap for the cross compilation toolchains
brew tap messense/macos-cross-toolchains
brew tap oxkitsune/macos-cross-libs

# Then we can install the toolchain and the required libraries
brew install llvm \ # llvm for the cross compilation toolchain
    x86_64-unknown-linux-gnu \ # The cross compilation toolchain
    x86_64-unknown-linux-gnu-alsa-lib \ # alsa library for audio
    nasm \ # The nasm assembler for libturbojpeg
```
