use clap::Parser;
use dummy_fuzzer::{Args, Fuzzer, Result};

fn main() -> Result<()> {
    let args = Args::parse();
    
    if args.target_cmd.is_empty() {
        return Err(dummy_fuzzer::FuzzerError::TargetExecution(
            "No target command specified".to_string()
        ));
    }

    let mut fuzzer = Fuzzer::new(args)?;
    fuzzer.run()
}
