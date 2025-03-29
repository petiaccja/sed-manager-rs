# SEDManager

![Language](https://img.shields.io/badge/Language-Rust-blue)
[![build_and_test](https://github.com/petiaccja/sed-manager-rs/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/petiaccja/sed-manager-rs/actions/workflows/build_and_test.yml)

An easy to use cross-platform GUI application for setting up self-encrypting drives. For more information and documentation, head to [SEDManager's website](https://petiaccja.github.io/sed-manager-website/).

## Introduction

SEDManager helps you quickly set up your TCG-compliant self-encrypting drives (SEDs). It also comes with a [pre-boot authentication environment](https://github.com/petiaccja/sed-manager-pba) (PBA) that lets you enter your password before booting your operating system.

### Supported encryption standards:

| TCG SSC      | Support      | Notes                                            |
|--------------|--------------|--------------------------------------------------|
| Enterprise   | Partial      | Detect, take ownership, activate locking, revert |
| Opal         | Full         | Opal 1.0 & 2.0 supported                         |
| Opalite      | Full         |                                                  |
| Pyrite       | Full         | Pyrite 1.0 & 2.0 supported                       |
| Ruby         | Full         |                                                  |
| Key per I/O  | Partial      | Detect, take ownership, activate locking, revert |

### Supported operating systems and interfaces:

|             | NVMe | SCSI | ATA/SATA |
|-------------|------|------|----------|
| Windows     | Yes  | Yes  | Yes      |
| Linux       | Yes  | No   | No       |
| PBA (Linux) | Yes  | No   | No       |

## Installation and usage

### Downloading the executables and the PBA

You can get the precompiled binaries for Linux and Windows on the [releases page](https://github.com/petiaccja/sed-manager-rs/releases), and you can get the ISO image for the PBA from a [separate repository](https://github.com/petiaccja/sed-manager-pba/releases). You can also [build](#building) SEDManager from sources. There is no installer, just unzip the files.

### Running as administrator/root

SEDManager will run without root access, but it needs access to raw disks (e.g. `/dev/nvme0` or `\\.\PhysicalDrive1`) to do anything useful. You can either run as administrator/root, or, if your OS makes it possible, give fine-grained access to one or more raw devices.

### Configuring your drives & system

If you're already familiar with self-encrypting drives and TCG specifications, like Opal, you can probably do it without further reading. If you aren't, head to the [website](https://petiaccja.github.io/sed-manager-website/) to read more.

### A word of warning

Before you jump in and start carelessly clicking around to encrypt your drive, you should be aware that it's very easy to **delete all your data**. Be sure you know what you're doing and read the warning messages.

## Building

SEDManager is a Rust application and uses Cargo for building.

This roughly translates to:
```sh
git clone https://github.com/petiaccja/sed-manager-rs.git
cd sed-manager-rs
cargo build
```

You will need to install the Rust toolchain, and possibly C/C++ compilers for some dependencies (e.g. Skia). You can also choose between the release (`--profile release`) and debug (`--profile dev`) profiles when using `cargo`.

## Attributions

SEDManager was built using:
- [Slint](https://slint.dev/): the entire UI is written using Slint
- [Material Symbols & Icons](https://fonts.google.com/icons): the UI uses Material design icons

## License

SEDManager is copyrighted while I'm working out the licensing terms, meaning you can not legally use either the sources or the binaries. Stay tuned.
