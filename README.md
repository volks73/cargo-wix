# cargo-wix: A cargo subcommand to create Windows installers

A subcommand for [Cargo](http://doc.crates.io/) that builds a Windows installer (msi) using the [Wix Toolset](http://wixtoolset.org/) from the release build of a [Rust](https://www.rust-lang.org) binary project. It also supports signing the Windows installer if a code signing certificate is available using the [SignTool](https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx) application available in the [Windows 10 SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk).

[![Crates.io](https://img.shields.io/crates/v/cargo-wix.svg)](https://crates.io/crates/cargo-wix)
[![GitHub release](https://img.shields.io/github/release/volks73/cargo-wix.svg)](https://github.com/volks73/cargo-wix/releases)
[![Crates.io](https://img.shields.io/crates/l/cargo-wix.svg)](https://github.com/volks73/cargo-wix#license)
[![Build Status](https://travis-ci.org/volks73/cargo-wix.svg?branch=master)](https://travis-ci.org/volks73/cargo-wix)

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
C:\> cargo install cargo-wix
```

## Usage

Start a command prompt, such as `cmd.exe`, the [Developer Prompt](https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx) installed with the [VC Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2017) (recommended), or [git bash](https://gitforwindows.org/), and navigate to the project's root folder. Run the subcommand:

```dos
C:\Path\to\Project> cargo wix init
```

This will create the `wix` folder in the project's root (along side the `Cargo.toml` file) and then it will create the `wix\main.wxs` file from the WiX Source (wxs) embedded within the subcommand. The generated `wix\main.wxs` file can be used without modification with the following command to create an installer for the project:

```dos
C:\Path\to\Project> cargo wix
```

The `cargo wix` subcommand without any arguments searches for a `wix\main.wxs` file, relative to the project's root. It will compile the `wix\main.wxs` file and then link the object file (`target\wix\build\main.wixobj`) to create the Windows installer (msi). The installer will be located in the `target\wix` folder. All artifacts of the installer compilation and linking process are placed within the `target\wix` folder. Paths in the `wix\main.wxs` file should be relative to the project's root, i.e. the same location as the `Cargo.toml` manifest file. 

A different WiX Source (wxs) file from the `wix\main.wxs` file can be used by specifying a path to it as an argument to the subcommand as follows:

```dos
C:\Path\to\Project> cargo wix Path\to\WiX\Source\File.wxs
```

The `print <template>` subcommand, which prints one of the embedded templates to stdout, can be used to create the `main.wxs` file. A [WXS template](https://github.com/volks73/cargo-wix/blob/master/src/main.wxs.mustache) file specifically designed to work with this subcommand is embedded within the `cargo-wix` binary during installation. Use the following commands to create a WiX Source file and use it to create an installer with this subcommand.

```dos
C:\Path\to\Project> cargo wix print wxs > example.wxs
C:\Path\to\Project> cargo wix example.wxs
```

The WiX source file can be customized using a text editor, but modification of the XML preprocessor variables should be avoided to ensure the `cargo wix` command works properly. 

To sign the installer (msi) as part of the build process, ensure the `signtool` command is available in the PATH system environment variable or use the [Developer Prompt](https://msdn.microsoft.com/en-us/library/f35ctcxw.aspx) that was installed with the Windows 10 SDK, and use the `sign` sub-subcommand as follows: 

```dos
C:\Path\to\Project> cargo wix sign
```

Use the `-h,--help` flag to display information about additional options and features.

```dos
C:\Path\to\Project> cargo wix -h
```

## Tests

The tests must be run using the `cargo test --all-targets -- --test-threads=1` command from the root folder of the project, i.e. the same location as the `Cargo.toml` file. The `--test-threads=1` option is needed because integration tests ran in parallel will cause many tests to fail. This is because many of the integration tests change the current working directory (CWD) to as closely as possible mimic usage by a user from within a cargo-based project. The same environment is shared across each test even though each test is essentially a separate application.

There are set environment variables that can be used to help debug a failing test. The `CARGO_WIX_TEST_PERSIST` environment variable can be set to persist the temporary directories that are created during integration tests. This allows the developer to inspect the contents of the temporary directory to better understand what the test was doing. The `CARGO_WIX_TEST_PERSIST` environment variable accepts any value. Unsetting the environment variable will delete the temporary directories after each test. The `CARGO_WIX_TEST_LOG` environment variable is sets the log level while running an integration test. It accepts an integer value between 0 and 5, with 0 turning off logging, and 5 displaying all log statements (ERROR, WARN, INFO, DEBUG, and TRACE). Log statements are __not__ captured during tests, so this environment variable should be used only when running an integration test in isolation to prevent "swampping" the terminal/console with statements. Finally, the `CARGO_WIX_TEST_NO_CAPTURE` environment variable accepts any value and will display the output from the WiX Toolset compiler (candle.exe) and linker (light.exe) when running an integration test. Similar to the `CARGO_WIX_TEST_LOG` environment variable, this variable should only be used in isolation to prevent "swamping" the terminal/console with the output from the WiX Toolset commands. By default, the output is captured by the _test_ not cargo's test framework; thus, the `cargo test -- --nocapture` command has no affect. Below is a [Powershell] example of debugging a failed integration test:

```powershell
PS C:\Path\to\Cargo\Wix> $env:CARGO_WIX_TEST_PERSIST=1; $env:CARGO_WIX_TEST_LOG=5; $env:CARGO_WIX_TEST_NO_CAPTURE=1; 
PS C:\Path\to\Cargo\Wix> cargo test <TEST NAME>
PS C:\Path\to\Cargo\Wix> Remove-Item Env:\CARGO_WIX_TEST_PERSIST; Remove-Item Env:\CARGO_WIX_TEST_LOG; Remove-Item Env:\CARGO_WIX_TEST_NO_CAPTURE
```

where `<TEST NAME>` is replaced with the name of an integration tests. The third line is optional and unsets the three environment variables to avoid additional tests from also persisting, logging, and dumping output to the terminal/console. Note, the `-- --nocapture` option is _not_ needed to display the logging statements or the output from the WiX Toolset compiler (candle.exe) and linker (light.exe).

[Powershell]: https://docs.microsoft.com/en-us/powershell/

## License

The `cargo-wix` project is licensed under either the [MIT license](https://opensource.org/licenses/MIT) or [Apache 2.0 license](http://www.apache.org/licenses/LICENSE-2.0). See the [LICENSE-MIT](https://github.com/volks73/cargo-wix/blob/master/LICENSE-MIT) or [LICENSE-APACHE](https://github.com/volks73/cargo-wix/blob/master/LICENSE-APACHE) files for more information about licensing and copyright.

