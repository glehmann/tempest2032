use std::{env, os::unix::process::CommandExt, process::Command};

fn main() {
    let executable = env::current_exe()
        .expect("could not locate the application launcher")
        .with_file_name("tempest2032");
    let error = Command::new(executable)
        .arg("--fullscreen")
        .args(env::args_os().skip(1))
        .exec();
    eprintln!("Could not start Tempest 2032: {error}");
    std::process::exit(1);
}
