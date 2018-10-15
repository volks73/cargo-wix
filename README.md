# cargo-wix: A cargo subcommand to create Windows installers

A subcommand for [Cargo](http://doc.crates.io/) that builds a Windows installer (msi) using the [Wix Toolset](http://wixtoolset.org/) from the release build of a [Rust](https://www.rust-lang.org) binary project. It also supports signing the Windows installer if a code signing certificate is available using the [SignTool](https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx) application available in the [Windows 10 SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk).

[![Crates.io](https://img.shields.io/crates/v/cargo-wix.svg)](https://crates.io/crates/cargo-wix)
[![GitHub release](https://img.shields.io/github/release/volks73/cargo-wix.svg)](https://github.com/volks73/cargo-wix/releases)
[![Crates.io](https://img.shields.io/crates/l/cargo-wix.svg)](https://github.com/volks73/cargo-wix#license)

## Quick Start

Start a command prompt (cmd.exe) and then execute the following commands:

```dos
C:\>cargo install cargo-wix
C:\>cd Path\To\Project
C:\Path\To\Project\>cargo wix init
C:\Path\To\Project\>cargo wix
```

The Windows installer (msi) for the project will be in the `C:\Path\To\Project\target\wix` folder.

## Installation

The cargo-wix project can be installed on any platform supported by the Rust programming language, but the Wix Toolset is Windows only; thus, this project is only useful when installed on a Windows machine. Ensure the following dependencies are installed before proceeding. Note, Cargo is installed automatically when installing the Rust programming language. The `stable-x86_64-pc-windows-msvc` toolchain is recommended.

- [Cargo](http://doc.crates.io)
- [Rust](https://www.rust-lang.org)
- [WiX Toolset](http://wixtoolset.org)
- [Windows 10 SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk) (Optional), needed for signing the installer

After installing and configuring the dependencies, execute the following command to install the `cargo-wix` subcommand:

```dos
C:\>cargo install cargo-wix
```

## Usage

Start a command prompt, such as `cmd.exe`, the [Developer Prompt](https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx) installed with the [VC Build Tools](http://landinghub.visualstudio.com/visual-cpp-build-tools) (recommended), or [git bash](https://gitforwindows.org/), and navigate to the project's root folder. Run the subcommand:

```dos
C:\Path\To\Project\>cargo wix init
```

This will create the `wix` folder in the project's root (along side the `Cargo.toml` file) and then it will create the `wix\main.wxs` file from the WiX Source (wxs) embedded within the subcommand. The generated `wix\main.wxs` file can be used without modification with the following command to create an installer for the project:

```dos
C:\Path\To\Project\>cargo wix
```

The `cargo wix` subcommand without any arguments searches for a `wix\main.wxs` file, relative to the project's root. It will compile the `wix\main.wxs` file and then link the object file (`target\wix\build\main.wixobj`) to create the Windows installer (msi). The installer will be located in the `target\wix` folder. All artifacts of the installer compilation and linking process are placed within the `target\wix` folder. Paths in the `wix\main.wxs` file should be relative to the project's root, i.e. the same location as the `Cargo.toml` manifest file. 

A different WiX Source (wxs) file from the `wix\main.wxs` file can be used by specifying a path to it as an argument to the subcommand as follows:

```dos
C:\Path\To\Project\>cargo wix Path\To\WiX\Source\file.wxs
```

The `print <template>` subcommand, which prints one of the embedded templates to stdout, can be used to create the `main.wxs` file. A [WXS template](https://github.com/volks73/cargo-wix/blob/master/src/main.wxs.mustache) file specifically designed to work with this subcommand is embedded within the `cargo-wix` binary during installation. Use the following commands to create a WiX Source file and use it to create an installer with this subcommand.

```dos
C:\Path\To\Project\>cargo print wxs > example.wxs
C:\Path\To\Project\>cargo wix example.wxs
```

The WiX source file can be customized using a text editor, but modification of the XML preprocessor variables should be avoided to ensure the `cargo wix` command works properly. 

To sign the installer (msi) as part of the build process, ensure the `signtool` command is available in the PATH system environment variable or use the [Developer Prompt](https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx) that was installed with the Windows 10 SDK, and use the `--sign` flag with the subcommand as follows: 

```dos
C:\Path\To\Project\>cargo wix sign
```

Use the `-h,--help` flag to display information about additional options and features.

```dos
C:\Path\To\Project\>cargo wix -h
```

## Tests

The tests can be run using the `cargo test -- --test-threads=1` command from the root folder of the project, i.e. the same location as the `Cargo.toml` file. The `--test-threads=1` option appears to be needed when running the integration tests because sometimes if the integration tests are run in parallel (default without the option) some of the tests will fail. I believe this is related to creating temporary test projects using Cargo. There appears to be some kind of race condition that causes Cargo to not create separate projects for each of the integration tests.

## License

The `cargo-wix` project is licensed under either the [MIT license](https://opensource.org/licenses/MIT) or [Apache 2.0 license](http://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE-MIT](https://github.com/volks73/cargo-wix/blob/master/LICENSE-MIT) or [LICENSE-APACHE](https://github.com/volks73/cargo-wix/blob/master/LICENSE-APACHE) files for more information about licensing and copyright.

