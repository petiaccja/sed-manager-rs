# SEDManager

![Language](https://img.shields.io/badge/Language-Rust-blue)
![Language](https://img.shields.io/badge/License-Proprietary-blue)
[![build_and_test](https://github.com/petiaccja/sed-manager-rs/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/petiaccja/sed-manager-rs/actions/workflows/build_and_test.yml)

An easy to use cross-platform GUI application for setting up self-encrypting drives (SEDs). Head to [SEDManager's website](https://sedmanager.app) for more information, downloads, and documentation.

SEDManager is free for non-commercial use. Read more about the [licensing terms](#license) below.

## Introduction

SEDManager focuses on setting up SEDs implemented to the [TCG storage specification](https://www.trustedcomputinggroup.org/work-groups/storage). Besides the configurator app, it comes with a [pre-boot authentication environment](https://github.com/petiaccja/sed-manager-pba) (PBA) that lets you enter your drive encryption password before booting your operating system.

### Supported encryption standards:

| TCG SSC     | Support | Notes                                            |
| ----------- | ------- | ------------------------------------------------ |
| Enterprise  | Partial | Detect, take ownership, activate locking, revert |
| Opal        | Full    | Opal 1.0 & 2.0 supported                         |
| Opalite     | Full    |                                                  |
| Pyrite      | Full    | Pyrite 1.0 & 2.0 supported                       |
| Ruby        | Full    |                                                  |
| Key per I/O | Partial | Detect, take ownership, activate locking, revert |

### Supported operating systems and interfaces:

|                   | NVMe | SCSI | ATA/SATA |
| ----------------- | ---- | ---- | -------- |
| Windows           | Yes  | Yes  | Yes      |
| Linux             | Yes  | No   | No       |
| PBA (Linux-based) | Yes  | No   | No       |

## Installation and usage

### Downloading the executables and the PBA

You can get the precompiled binaries for Linux and Windows on the [releases page](https://github.com/petiaccja/sed-manager-rs/releases), and you can get the ISO image for the PBA from a [separate repository](https://github.com/petiaccja/sed-manager-pba/releases). You can also [build](#building) SEDManager from sources. There is no installer, just unzip the files.

### Setting up elevated privileges

SEDManager needs to issue I/O control commands (ioctls) to your storage drivers, and that requires additional privileges.

- On Windows: run SEDManager as administrator.
- On Linux: use the scripts in the `install` folder. One sets up udev rules to grant access to block devices for your user, the other sets additional capabilities on the executable that are needed to issue ioctls. Do not run as root.

SEDManager will run without privileges too, but it won't be able to open physical disks.

### Configuring your drives

If you're already familiar with self-encrypting drives and TCG specifications, like Opal, you can probably do it without further reading. If you aren't, head to the [website](https://petiaccja.github.io/sed-manager-website/) to learn about the concepts.

> [!WARNING]
> **You can easily erase all your data!** Make sure you know what you are doing, heed the warning messages, and back up your data before configuring your drives.

## Building

SEDManager is a standard Rust application that you can build with `cargo`:

```sh
sudo apt-get install libfontconfig-dev
sudo apt-get install libfreetype-dev
git clone https://github.com/petiaccja/sed-manager-rs.git
cd sed-manager-rs
cargo build
```

On Linux, you will have to install development headers for `libfontconfig` and `libfreetype`; there is nothing to do on Windows.

Alternatively, you can use the devcontainer.

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
- Added a small feature ⇒ open a pull request

Before starting work on PR, it's a good idea to get in touch by opening an issue or starting a discussion. This way, we can make sure your work aligns with the development direction and we have a better chance to accept your PR.

AI-assisted contributions are accepted, but manual review for every line is required from the author prior to submission. Vibe-coded, unchecked, or slop PRs will be closed and you'll be banned from contributing.

## Attributions

SEDManager was built using:

- [Slint](https://slint.dev/): a handy GUI library powering SEDManager's interface.
- [Material Symbols & Icons](https://fonts.google.com/icons): aesthetic icons used across SEDManager's interface.
- Many other libraries.

Thanks!
