fn main()
{
    let mozjs =
        pkg_config::Config::new()
        .probe("mozjs-91")
        .unwrap();

    let backend_cpp = "src/backend/backend.cpp";

    println!("cargo:rerun-if-changed={backend_cpp}");

    cc::Build::new()
        .cpp(true)
        .flag("-std=c++20")
        .includes(mozjs.include_paths)
        .file(backend_cpp)
        .compile("sekka_backend");
}
