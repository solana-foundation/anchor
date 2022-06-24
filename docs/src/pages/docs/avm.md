---
title: Anchor Version Manager
description: Anchor - Anchor Version Manager
---

Anchor Version Manager (avm) is provided to manage multiple installations of the anchor-cli binary. This may be required to produce verifiable builds, or if you'd prefer to work with an alternate version.

---

```shell
Anchor version manager

USAGE:
    avm <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    help         Print this message or the help of the given subcommand(s)
    install      Install a version of Anchor
    list         List available versions of Anchor
    uninstall    Uninstall a version of Anchor
    use          Use a specific version of Anchor
```

## Install

```shell
avm install <version>
```

Install the specified version of anchor-cli. The version argument should follow semver versioning. It is also possible to use `latest` as the version argument to install the latest version.

## List

```shell
avm list
```

Lists available versions of anchor-cli.

```shell
0.3.0
0.4.0
0.4.1
0.4.2
0.4.3
0.4.4
0.4.5
0.5.0
0.6.0
0.7.0
0.8.0
0.9.0
0.10.0
0.11.0
0.11.1
0.12.0
0.13.0
0.13.1
0.13.2
0.14.0
0.15.0
0.16.0
0.16.1
0.16.2
0.17.0
0.18.0
0.18.2
0.19.0
0.20.0  (installed)
0.20.1  (latest, installed, current)
```

## Uninstall

```shell
avm uninstall <version>
```

## Use

```shell
avm use <version>
```

Use a specific version. This version will remain in use until you change it by calling the same command again. Similarly to `avm install`, you can also use `latest` for the version.
