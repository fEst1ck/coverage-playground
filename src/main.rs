use clap::Parser;
use dummy_fuzzer::{Args, Fuzzer};
use std::process;
use log::error;
use env_logger;

fn main() {
    // Initialize the logger
    env_logger::init();

    let args = Args::parse();
    
    if args.target_cmd.is_empty() {
        error!("No target command specified");
        process::exit(1);
    }

    match Fuzzer::new(args) {
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
