# yggdrasil 🌲
yggdrasil is the robot framework built by the Dutch Nao Team for the SPL competition. 

## Building
yggdrasil supports building for both linux and macos. Building on windows is not supported, but it is possible to build using WSL.

Once all required dependencies have been installed, building is as simple as running the following command:

```bash
./sindri run <robot-number>
```

## Dependencies
yggdrasil is built using Rust, and as such requires the Rust toolchain to be installed.
We recommend installing it using rustup, which can be found [here](https://rustup.rs/).

The robots run our own Arch linux based distribution, and as such the `x86_64-unknown-linux-gnu` target is required for cross compilation:

```bash
rustup target add x86_64-unknown-linux-gnu
```

### Linux
Building for linux is incredibly simple, as it only requires the following dependencies:

**Ubuntu**
```bash
sudo apt-get install libasound2-dev libv4l-dev nasm
```

**Arch**
```bash
sudo pacman -S alsa-lib v4l-utils nasm
```

### macOS
Building for macOS is supported for both ARM/Intel macs, but it does require a cross compilation toolchain. 

Both the toolchain and the required libraries can be installed using homebrew:

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

