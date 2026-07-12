fn main() {
    if let Err(error) = darwinbots_cli::run_from(std::env::args_os(), std::io::stdout()) {
        eprintln!("darwinbots: {error}");
        std::process::exit(1);
    }
}

