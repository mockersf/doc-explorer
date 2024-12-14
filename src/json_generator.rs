use xshell;

pub fn generate_jsons(krate: String) {
    let _ = std::fs::remove_dir_all("./jsons");
    let shell = xshell::Shell::new().unwrap();
    shell
        .cmd("cargo")
        .args(["new", "tmp-crate"])
        .quiet()
        .ignore_stderr()
        .ignore_stdout()
        .run()
        .unwrap();

    shell.change_dir("tmp-crate");
    shell
        .cmd("cargo")
        .args(["add", krate.as_str()])
        .quiet()
        .ignore_stderr()
        .ignore_stdout()
        .run()
        .unwrap();
    shell
        .cmd("cargo")
        .env("RUSTDOCFLAGS", "-Z unstable-options --output-format json")
        .args(["+nightly", "doc"])
        .quiet()
        .ignore_stderr()
        .ignore_stdout()
        .run()
        .unwrap();
    std::fs::rename("./tmp-crate/target/doc", "./jsons").unwrap();
    std::fs::remove_dir_all("./tmp-crate").unwrap();
}
