fn main()
{
    let backend_cpp = "src/backend/backend.cpp";

    println!("cargo:rerun-if-changed={backend_cpp}");

    cc::Build::new()
        .cpp(true)
        .flag("-std=c++14")
        .file(backend_cpp)
        .compile("sekka_backend");

    // v8 must be linked after sekka_backend,
    // so we cannot use `#[link(name = "v8")]`.
    println!("cargo:rustc-link-lib=v8");
}
