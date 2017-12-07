var searchIndex = {};
searchIndex["cargo_wix"] = {"doc":"cargo-wix","items":[[3,"Wix","cargo_wix","The builder for running the subcommand.",null,null],[4,"Error","","",null,null],[13,"Command","","A command operation failed.",0,null],[13,"Generic","","A generic or custom error occurred. The message should contain the detailed information.",0,null],[13,"Io","","An I/O operation failed.",0,null],[13,"Manifest","","A needed field within the `Cargo.toml` manifest could not be found.",0,null],[13,"Mustache","","An error occurred with rendering the template using the mustache renderer.",0,null],[13,"Toml","","Parsing of the `Cargo.toml` manifest failed.",0,null],[4,"Platform","","The different values for the `Platform` attribute of the `Package` element.",null,null],[13,"X86","","The `x86` WiX Toolset value.",1,null],[13,"X64","","The `x64` WiX Toolset value.",1,null],[5,"print_template","","Generates unique GUIDs for appropriate values in the template and prints to stdout.",null,{"inputs":[],"output":{"name":"result"}}],[5,"init","","Creates the necessary sub-folders and files to immediately use the `cargo wix` subcommand to create an installer for the package.",null,{"inputs":[{"name":"bool"}],"output":{"name":"result"}}],[11,"fmt","","",0,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"code","","Gets an error code related to the error.",0,{"inputs":[{"name":"self"}],"output":{"name":"i32"}}],[11,"description","","",0,{"inputs":[{"name":"self"}],"output":{"name":"str"}}],[11,"cause","","",0,{"inputs":[{"name":"self"}],"output":{"name":"option"}}],[11,"fmt","","",0,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"from","","",0,{"inputs":[{"name":"error"}],"output":{"name":"error"}}],[11,"from","","",0,{"inputs":[{"name":"error"}],"output":{"name":"error"}}],[11,"from","","",0,{"inputs":[{"name":"error"}],"output":{"name":"error"}}],[11,"fmt","","",1,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"clone","","",1,{"inputs":[{"name":"self"}],"output":{"name":"platform"}}],[11,"eq","","",1,{"inputs":[{"name":"self"},{"name":"platform"}],"output":{"name":"bool"}}],[11,"arch","","Gets the name of the platform as an architecture string as used in Rust toolchains.",1,{"inputs":[{"name":"self"}],"output":{"name":"str"}}],[11,"fmt","","",1,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"default","","",1,{"inputs":[],"output":{"name":"self"}}],[11,"fmt","","",2,{"inputs":[{"name":"self"},{"name":"formatter"}],"output":{"name":"result"}}],[11,"clone","","",2,{"inputs":[{"name":"self"}],"output":{"name":"wix"}}],[11,"new","","Creates a new `Wix` instance.",2,{"inputs":[],"output":{"name":"self"}}],[11,"binary_name","","Sets the binary name.",2,{"inputs":[{"name":"self"},{"name":"option"}],"output":{"name":"self"}}],[11,"capture_output","","Enables or disables capturing of the output from the builder (`cargo`), compiler (`candle`), linker (`light`), and signer (`signtool`).",2,{"inputs":[{"name":"self"},{"name":"bool"}],"output":{"name":"self"}}],[11,"description","","Sets the description.",2,{"inputs":[{"name":"self"},{"name":"option"}],"output":{"name":"self"}}],[11,"input","","Sets the path to a file to be used as the WiX Source (wxs) file instead of `wix\\main.rs`.",2,{"inputs":[{"name":"self"},{"name":"option"}],"output":{"name":"self"}}],[11,"manufacturer","","Overrides the first author in the `authors` field of the package's manifest (Cargo.toml) as the manufacturer within the installer.",2,{"inputs":[{"name":"self"},{"name":"option"}],"output":{"name":"self"}}],[11,"product_name","","Sets the product name.",2,{"inputs":[{"name":"self"},{"name":"option"}],"output":{"name":"self"}}],[11,"sign","","Enables or disables signing of the installer after creation with the `signtool` application.",2,{"inputs":[{"name":"self"},{"name":"bool"}],"output":{"name":"self"}}],[11,"timestamp","","Sets the URL for the timestamp server used when signing an installer.",2,{"inputs":[{"name":"self"},{"name":"option"}],"output":{"name":"self"}}],[11,"run","","Runs the subcommand to build the release binary, compile, link, and possibly sign the installer (msi).",2,{"inputs":[{"name":"self"}],"output":{"name":"result"}}],[11,"default","","",2,{"inputs":[],"output":{"name":"self"}}]],"paths":[[4,"Error"],[4,"Platform"],[3,"Wix"]]};
initSearch(searchIndex);
