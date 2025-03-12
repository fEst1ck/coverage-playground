use anyhow::Result;
use clap::Parser;
use std::{ffi::OsString, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(trailing_var_arg = false)]
#[command(arg_required_else_help = true)]
pub struct Args {
    /// Input seeds directory
    #[arg(short = 'i', long)]
    pub input_dir: PathBuf,

    /// Output directory for findings
    #[arg(short = 'o', long)]
    pub output_dir: PathBuf,

    /// Coverage types to use (comma-separated: block, edge, path)
    #[arg(short = 'c', long, default_value = "block", value_delimiter = ',')]
    pub coverage_types: Vec<String>,

    /// Enable advanced mode
    #[arg(short = 'a', long, default_value = "false")]
    pub all_coverage: bool,

    /// Target command and its arguments (after --)
    #[arg(last = true, required = true, allow_hyphen_values = false)]
    pub target_cmd: Vec<OsString>,
}

impl Args {
    pub fn validate(&self) -> Result<()> {
        if self.coverage_types.is_empty() {
            anyhow::bail!("At least one coverage type must be specified");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Args {
        Args::try_parse_from(args).unwrap_or_else(|e| panic!("{}", e))
    }

    #[test]
    fn test_minimal_valid_args() {
        let args = parse_args(&["fuzzer", "-i", "/seeds", "-o", "/output", "--", "target"]);

        assert_eq!(args.input_dir.to_str().unwrap(), "/seeds");
        assert_eq!(args.output_dir.to_str().unwrap(), "/output");
        assert_eq!(args.coverage_types, vec![String::from("block")]);
        assert_eq!(args.target_cmd.len(), 1);
        assert_eq!(args.target_cmd[0].to_str().unwrap(), "target");
        assert_eq!(args.all_coverage, false);
    }

    #[test]
    fn test_full_valid_args() {
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/path/to/seeds",
            "-o",
            "/path/to/output",
            "-c",
            "edge",
            "--",
            "./target",
            "-f",
            "@@",
            "--verbose",
            "--config=/etc/config",
        ]);

        assert_eq!(args.input_dir.to_str().unwrap(), "/path/to/seeds");
        assert_eq!(args.output_dir.to_str().unwrap(), "/path/to/output");
        assert_eq!(args.coverage_types, vec![String::from("edge")]);

        let target_args: Vec<_> = args
            .target_cmd
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(
            target_args,
            vec!["./target", "-f", "@@", "--verbose", "--config=/etc/config",]
        );
    }

    #[test]
    fn test_all_coverage_types() {
        // Test block coverage (default)
        let args = parse_args(&["fuzzer", "-i", "/seeds", "-o", "/out", "--", "target"]);
        assert_eq!(args.coverage_types, vec![String::from("block")]);

        // Test edge coverage
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/out", "-c", "edge", "--", "target",
        ]);
        assert_eq!(args.coverage_types, vec![String::from("edge")]);

        // Test path coverage
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/out", "-c", "path", "--", "target",
        ]);
        assert_eq!(args.coverage_types, vec![String::from("path")]);

        // Test multiple coverage types
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/out",
            "-c",
            "block,edge,path",
            "--",
            "target",
        ]);
        assert_eq!(
            args.coverage_types,
            vec![
                String::from("block"),
                String::from("edge"),
                String::from("path")
            ]
        );

        // Test subset of coverage types
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/out",
            "-c",
            "block,path",
            "--",
            "target",
        ]);
        assert_eq!(
            args.coverage_types,
            vec![String::from("block"), String::from("path")]
        );
    }

    #[test]
    #[should_panic]
    fn test_missing_input_dir() {
        parse_args(&["fuzzer", "-o", "/output", "--", "target"]);
    }

    #[test]
    #[should_panic]
    fn test_missing_output_dir() {
        parse_args(&["fuzzer", "-i", "/seeds", "--", "target"]);
    }

    #[test]
    #[should_panic]
    fn test_missing_target() {
        parse_args(&["fuzzer", "-i", "/seeds", "-o", "/output", "--"]);
    }

    #[test]
    #[should_panic]
    fn test_missing_separator() {
        let res = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/output", "target", // Missing -- before target
        ]);
        println!("Result: {:?}", res);
    }

    #[test]
    fn test_target_with_special_args() {
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/output",
            "--",
            "./target",
            "-i",
            "input.txt", // -i here is a target arg, not fuzzer arg
            "-o",
            "output.txt", // -o here is a target arg, not fuzzer arg
            "@@",
            "--", // Additional -- is part of target args
            "-x",
        ]);

        let target_args: Vec<_> = args
            .target_cmd
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect();
        assert_eq!(
            target_args,
            vec![
                "./target",
                "-i",
                "input.txt",
                "-o",
                "output.txt",
                "@@",
                "--",
                "-x",
            ]
        );
    }

    #[test]
    fn test_relative_paths() {
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "./seeds",
            "-o",
            "../output",
            "--",
            "./target",
        ]);

        assert_eq!(args.input_dir.to_str().unwrap(), "./seeds");
        assert_eq!(args.output_dir.to_str().unwrap(), "../output");
    }

    #[test]
    fn test_advanced_mode() {
        // Test without advanced mode (default)
        let args = parse_args(&["fuzzer", "-i", "/seeds", "-o", "/output", "--", "target"]);
        assert_eq!(args.all_coverage, false);

        // Test with advanced mode short flag
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/output", "-a", "--", "target",
        ]);
        assert_eq!(args.all_coverage, true);

        // Test with advanced mode long flag
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/output",
            "--all-coverage",
            "--",
            "target",
        ]);
        assert_eq!(args.all_coverage, true);
    }
}
