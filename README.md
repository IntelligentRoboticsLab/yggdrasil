# yggdrasil 🌲
yggdrasil is the robot framework built by the Dutch Nao Team for the SPL competition. 

## Building
yggdrasil supports building for both linux and macos. Building on windows is not supported, but it is possible to build using WSL.

### Linux
Building for linux is incredibly simple, as it only requires the following dependencies:
- alsa
- v4l2
- nasm

#### Ubuntu
```bash
sudo apt-get install libasound2-dev libv4l-dev nasm
```

### Arch
```bash
sudo pacman -S alsa-lib v4l-utils nasm
```

### macos
Building for macos is supported for both arm/intel macs, but it does require a cross compilation toolchain. 

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
