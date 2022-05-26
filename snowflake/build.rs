fn main()
{
    let perform_run_command_cpp = "src/action/perform/run_command.cpp";

    println!("cargo:rerun-if-changed={perform_run_command_cpp}");

    cc::Build::new()
        .cpp(true)
        .file(perform_run_command_cpp)
        .compile("snowflake_perform_run_command");
}
