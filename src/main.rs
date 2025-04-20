use clap::Parser;
use dummy_fuzzer::{
    cli::Args,
    fuzzer::{parallel::ParallelFuzzer, Fuzzer},
};
use env_logger;
use log::{error, info};
use std::process;

fn main() {
    // Initialize the logger
    env_logger::init();

    let args = Args::parse();

    info!("Starting fuzzer with args: {:?}", args);

    if args.num_instances > 1 {
        // Use parallel fuzzing
        match ParallelFuzzer::new(args.clone(), args.num_instances) {
            Ok(mut parallel_fuzzer) => {
                if let Err(e) = parallel_fuzzer.run() {
                    error!("Error running parallel fuzzer: {}", e);
                    process::exit(1);
                }
            }
            Err(e) => {
                error!("Error creating parallel fuzzer: {}", e);
                process::exit(1);
            }
        }
    } else {
        // Use single instance fuzzing
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
}
