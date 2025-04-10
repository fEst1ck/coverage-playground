use clap::Parser;
use dummy_fuzzer::{Args, Fuzzer};
use env_logger;
use log::error;
use std::process;

fn main() {
    // Initialize the logger
    env_logger::init();

    let args = Args::parse();

    if args.target_cmd.is_empty() {
        error!("No target command specified");
        process::exit(1);
    }

    match Fuzzer::new(args, 0) {
        Ok(mut fuzzer) => {
            if let Err(e) = fuzzer.run() {
                error!("Error running fuzzer: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            error!("Error creating fuzzer: {}", e);
            process::exit(1);
        }
    }
}
