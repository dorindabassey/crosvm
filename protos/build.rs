// Copyright 2019 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct ExternalProto {
    // Where to find protos during builds within cros_sdk. Relative to
    // $SYSROOT environment variable set by emerge builds.
    dir_relative_to_sysroot: &'static str,

    // Where to find protos during "cargo build" in a local developer
    // environment. Relative to the platform/crosvm/protos directory.
    dir_relative_to_us: &'static str,

    // *.proto file expected to exist in both of the above directories.
    proto_file_name: &'static str,

    // Code generated by proto compiler will be placed under
    // protos::generated::$module_name.
    module: &'static str,
}

// Rustfmt bug: https://github.com/rust-lang/rustfmt/issues/3498
#[rustfmt::skip]
static EXTERNAL_PROTOS: &[ExternalProto] = &[
    #[cfg(feature = "trunks")]
    ExternalProto {
        dir_relative_to_sysroot: "usr/include/chromeos/dbus/trunks",
        dir_relative_to_us: "../../../platform2/trunks",
        proto_file_name: "interface.proto",
        module: "trunks",
    },
];

struct LocalProto {
    // Corresponding to the input file src/$module.proto.
    module: &'static str,
}

#[rustfmt::skip]
static LOCAL_PROTOS: &[LocalProto] = &[
    #[cfg(feature = "plugin")]
    LocalProto { module: "plugin" },
];

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR")?;
    let sysroot = env::var_os("SYSROOT");

    // Write out a Rust module that imports the modules generated by protoc.
    let generated = PathBuf::from(&out_dir).join("generated.rs");
    let out = File::create(generated)?;

    // Compile external protos.
    for proto in EXTERNAL_PROTOS {
        let dir = match &sysroot {
            Some(dir) => PathBuf::from(dir).join(proto.dir_relative_to_sysroot),
            None => PathBuf::from(proto.dir_relative_to_us),
        };
        let input_path = dir.join(proto.proto_file_name);
        protoc(proto.module, input_path, &out)?;
    }

    // Compile protos from the local src directory.
    for proto in LOCAL_PROTOS {
        let input_path = format!("src/{}.proto", proto.module);
        protoc(proto.module, input_path, &out)?;
    }

    Ok(())
}

// Compile a single proto file located at $input_path, placing the generated
// code at $OUT_DIR/$module and emitting the right `pub mod $module` into the
// output file.
fn protoc<P: AsRef<Path>>(module: &str, input_path: P, mut out: &File) -> Result<()> {
    let input_path = input_path.as_ref();
    let input_dir = input_path.parent().unwrap();

    // Place output in a subdirectory so that different protos with the same
    // common filename (like interface.proto) do not conflict.
    let out_dir = format!("{}/{}", env::var("OUT_DIR")?, module);
    fs::create_dir_all(&out_dir)?;

    // Invoke protobuf compiler.
    protoc_rust::run(protoc_rust::Args {
        out_dir: &out_dir,
        includes: &[input_dir.as_os_str().to_str().unwrap()],
        input: &[input_path.as_os_str().to_str().unwrap()],
        ..Default::default()
    })?;

    // Write out a `mod` that refers to the generated module.
    let file_stem = input_path.file_stem().unwrap().to_str().unwrap();
    writeln!(out, "#[path = \"{}/{}.rs\"]", out_dir, file_stem)?;
    writeln!(out, "pub mod {};", module)?;

    Ok(())
}
