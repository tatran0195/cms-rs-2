use std::{
    env,
    process::{self, Command},
};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("e2e") => run_e2e(args.collect()),
        Some("help") | None => {
            println!("Usage: cargo xtask e2e [-- <extra cargo test args>]");
            println!("Runs the PostgreSQL-backed product-flow integration test.");
        }
        Some(command) => {
            eprintln!("Unknown xtask command: {command}\nRun `cargo xtask help` for usage.");
            process::exit(2);
        }
    }
}

fn run_e2e(extra_test_args: Vec<String>) {
    if env::var_os("CMS_E2E_DATABASE_URL").is_none() {
        eprintln!(
            "CMS_E2E_DATABASE_URL is required and must point to a disposable PostgreSQL database.\
             \nExample: CMS_E2E_DATABASE_URL=postgres://postgres:postgres@localhost:5432/cms_e2e cargo xtask e2e"
        );
        process::exit(2);
    }

    let mut command = Command::new("cargo");
    command.args([
        "test",
        "-p",
        "cms-api",
        "--test",
        "product_flow_e2e",
        "--",
        "--ignored",
        "--nocapture",
    ]);
    command.args(extra_test_args);

    match command.status() {
        Ok(status) if status.success() => {}
        Ok(status) => process::exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!("Could not start cargo test: {error}");
            process::exit(1);
        }
    }
}
