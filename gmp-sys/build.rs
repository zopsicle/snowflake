#![feature(exit_status_error)]

use std::{env::var_os, path::Path, process::Command};

fn main()
{
    let out_dir = var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    Command::new("bindgen")
        .arg("--no-layout-tests")
        .arg("--output").arg(out_dir.join("bindgen.h.rs"))
        .arg("src/bindgen.h")
        .spawn().unwrap()
        .wait().unwrap()
        .exit_ok().unwrap();
}
