# cargo-wix: A cargo subcommand to create Windows installers

A subcommand for [Cargo](http://doc.crates.io/) that builds a Windows installer (msi) using the [Wix Toolset](http://wixtoolset.org/) from the release build of a [Rust](https://www.rust-lang.org) binary project. It also supports signing the Windows installer if a code signing certificate is available using the [SignTool](https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx) application available in the [Windows 10 SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk).

[![](http://meritbadge.herokuapp.com/cargo-wix)](https://crates.io/crates/cargo-wix)

## Quick Start

Start a command prompt (cmd.exe) and then execute the following commands:

```dos
C:\>cargo install cargo-wix
C:\>cd Path\To\Project
C:\Path\To\Project\>cargo wix --init
C:\Path\To\Project\>cargo wix
```

The Windows installer (msi) for the project will be in the `C:\Path\To\Project\target\wix` folder.

## Installation

The cargo-wix project can be installed on any platform supported by the Rust programming language, but the Wix Toolset is Windows only; thus, this project is only useful when installed on a Windows machine. Ensure the following dependencies are installed before proceeding. Note, Cargo is installed automatically when installing the Rust programming language. The `stable-x86_64-pc-windows-msvc` toolchain is recommended.

- [Cargo](http://doc.crates.io)
- [Rust](https://www.rust-lang.org)
- [WiX Toolset](http://wixtoolset.org)
- [Windows 10 SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk), needed for optionally signing the installer

The SignTool application is used to optionally sign an installer. It is available as part of the Windows 10 SDK.

__Note__, the WiX Toolset `bin` folder must be added to the PATH system environment variable for the `cargo wix` subcommand to work properly. The typical location for the WiX Toolset `bin` folder is: `C:\Program Files (x86)\WiX Toolset\bin`. Please add this path to the PATH system environment variable.

After installing and configuring the dependencies, execute the following command to install the `cargo-wix` subcommand:

```dos
C:\>cargo install cargo-wix
```

## Usage

__Important__, start and use the [Developer Prompt](https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx) that was installed with the [VC Build Tools](http://landinghub.visualstudio.com/visual-cpp-build-tools) or Windows 10 SDK. This will ensure the `signtool` command is available in the PATH environment variable if signing the installer with the `cargo-wix` subcommand. If not signing the installer, then any command prompt can be used. 

Navigate to the project's root folder and run the subcommand:

```dos
C:\Path\To\Project\>cargo wix --init
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

The `--print-wxs` flag, which prints a WiX Source (wxs) template to stdout, can be used to create the `main.wxs` file. A [template](https://github.com/volks73/cargo-wix/blob/master/src/main.wxs.mustache) file specifically designed to work with this subcommand is embedded within the `cargo-wix` binary during installation. Use the following commands to create a WiX Source file and use it to create an installer with this subcommand.

```dos
C:\Path\To\Project\>cargo --print-wxs > example.wxs
C:\Path\To\Project\>cargo wix example.wxs
```

The WiX source file can be customized using a text editor, but modification of the XML preprocessor variables should be avoided to ensure the `cargo wix` command works properly. 

To sign the installer (msi) as part of the build process, use the `--sign` flag with the subcommand as follows:

```dos
C:\Path\To\Project\>cargo wix --sign
```

Use the `-h,--help` flag to display information about additional options and features.

```dos
C:\Path\To\Project\>cargo wix -h
```

## License

The `cargo-wix` project is licensed under either the [MIT license](https://opensource.org/licenses/MIT) or [Apache 2.0 license](http://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE-MIT](https://github.com/volks73/cargo-wix/blob/master/LICENSE-MIT) or [LICENSE-APACHE](https://github.com/volks73/cargo-wix/blob/master/LICENSE-APACHE) files for more information about licensing and copyright.

