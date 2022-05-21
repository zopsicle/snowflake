#![feature(exit_status_error)]

use std::{env::var_os, path::PathBuf, process::Command};

fn main()
{
    let out_dir = PathBuf::from(var_os("OUT_DIR").unwrap());

    let in_path = "src/bindgen.h";
    let out_path = out_dir.join("bindgen.h.rs");

    println!("cargo:rerun-if-changed={in_path}");

    Command::new("bindgen")
        .arg("--default-macro-constant-type").arg("signed")
        .arg("--no-layout-tests")
        .arg("--output").arg(out_path)
        .arg(in_path)
        .spawn().unwrap()
        .wait().unwrap()
        .exit_ok().unwrap();
}
