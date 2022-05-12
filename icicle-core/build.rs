#![feature(exit_status_error)]

use std::{env::var_os, path::Path, process::Command};

fn main()
{
    let out_dir = var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let in_path = "src/heap/diagram.drawio";
    let out_path = out_dir.join("heap_diagram.drawio.svg");

    print!("cargo:rerun-if-changed={}", in_path);

    Command::new("drawio")
        .arg("--export")
        .arg("--output").arg(out_path)
        .arg("--transparent")
        .arg(in_path)
        .spawn().unwrap()
        .wait().unwrap()
        .exit_ok().unwrap();
}
