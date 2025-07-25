# SEDManager

![Language](https://img.shields.io/badge/Language-Rust-blue)
![Language](https://img.shields.io/badge/License-Proprietary-blue)
[![build_and_test](https://github.com/petiaccja/sed-manager-rs/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/petiaccja/sed-manager-rs/actions/workflows/build_and_test.yml)

An easy to use cross-platform GUI application for setting up self-encrypting drives. Head to [SEDManager's website](https://sedmanager.app) for more information, downloads, and documentation.

SEDManager is free for non-commercial use. Read more about the [licensing terms](#license) below.

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

|                   | NVMe | SCSI | ATA/SATA |
|-------------------|------|------|----------|
| Windows           | Yes  | Yes  | Yes      |
| Linux             | Yes  | No   | No       |
| PBA (Linux-based) | Yes  | No   | No       |

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

## License

SEDManager has a proprietary license. The key points:
- **Free for non-commercial users**. For example, encrypting a family computer.
- **Free for individual commercial users**. For example, you're a freelancer and you want to encrypt your work laptop.
- **Paid for general commercial users**. Anything other than the above two. Please [reach out](mailto:license@sedmanager.app) if you're interested in using SEDManager commercially.
- **You can edit the source code** in all cases, but there are limitations on how you can share your edits.

Please read the [full license](./LICENSE.md) for the exact terms. This short summary is not binding.

## Contributing

If you're interested in contributing, please do so via GitHub's interface:
- Found a bug ⇒ open an issue
- Fixed a bug ⇒ open a pull request
- Have a feature request ⇒ open an issue
- Added a feature ⇒ open an issue

If you're working on a larger feature or fix and you intend to contribute it to upstream, it's best to first open an issue to get in touch. An initial discussion will save you the trouble of working on something that cannot be accepted into upstream.

## Attributions

SEDManager was built using:
- [Slint](https://slint.dev/): a handy GUI library powering SEDManager's interface.
- [Material Symbols & Icons](https://fonts.google.com/icons): aesthetic icons used across SEDManager's interface.
