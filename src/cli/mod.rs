use anyhow::Result;
use clap::Parser;
use std::{ffi::OsString, path::PathBuf};

fn validate_coverage_type(s: &str) -> Result<String, String> {
    match s {
        "block" | "edge" | "path" => Ok(s.to_string()),
        _ => Err(format!("Invalid coverage type: {}", s)),
    }
}

#[derive(Parser, Debug, Clone)]
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

    /// Number of parallel fuzzer instances
    #[arg(short = 'j', long, default_value = "1")]
    pub num_instances: usize,

    /// Coverage types to use (comma-separated: block, edge, path)
    #[arg(short = 'c', long, default_value = "block", value_delimiter = ',')]
    pub coverage_types: Vec<String>,

    /// Coverage types to use (comma-separated: block, edge, path)
    #[arg(short = 'u', long, default_value = "block", value_delimiter = ',', value_parser = validate_coverage_type)]
    pub use_coverage: Vec<String>,

    /// Enable debug mode (prints additional information)
    #[arg(long)]
    pub debug: bool,

    /// Target program and its arguments
    #[arg(required = true, last = true, allow_hyphen_values = false)]
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
        assert_eq!(args.debug, false);
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
    fn test_debug_mode() {
        // Test without debug mode (default)
        let args = parse_args(&["fuzzer", "-i", "/seeds", "-o", "/output", "--", "target"]);
        assert_eq!(args.debug, false);

        // Test with debug mode
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/output", "--debug", "--", "target",
        ]);
        assert_eq!(args.debug, true);
    }

    #[test]
    fn test_use_coverage_types() {
        // Test block coverage (default)
        let args = parse_args(&["fuzzer", "-i", "/seeds", "-o", "/out", "--", "target"]);
        assert_eq!(args.use_coverage, vec![String::from("block")]);

        // Test edge coverage
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/out", "-u", "edge", "--", "target",
        ]);
        assert_eq!(args.use_coverage, vec![String::from("edge")]);

        // Test path coverage
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/out", "-u", "path", "--", "target",
        ]);
        assert_eq!(args.use_coverage, vec![String::from("path")]);

        // Test multiple coverage types
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/out",
            "-u",
            "block,edge,path",
            "--",
            "target",
        ]);
        assert_eq!(
            args.use_coverage,
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
            "-u",
            "block,path",
            "--",
            "target",
        ]);
        assert_eq!(
            args.use_coverage,
            vec![String::from("block"), String::from("path")]
        );
    }

    #[test]
    fn test_coverage_and_use_coverage_combination() {
        // Test different values for coverage and use_coverage
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/out",
            "-c",
            "block,edge,path",
            "-u",
            "edge,path",
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
        assert_eq!(
            args.use_coverage,
            vec![String::from("edge"), String::from("path")]
        );

        // Test when they're the same
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/out",
            "-c",
            "block,path",
            "-u",
            "block,path",
            "--",
            "target",
        ]);
        assert_eq!(args.coverage_types, args.use_coverage);
        assert_eq!(
            args.coverage_types,
            vec![String::from("block"), String::from("path")]
        );
    }

    #[test]
    #[should_panic]
    fn test_invalid_use_coverage_type() {
        parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/output", "-u", "invalid", "--", "target",
        ]);
    }

    #[test]
    #[should_panic]
    fn test_invalid_use_coverage_type_in_list() {
        parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/output",
            "-u",
            "block,invalid,path",
            "--",
            "target",
        ]);
    }

    #[test]
    fn test_num_instances() {
        // Test default (1 instance)
        let args = parse_args(&["fuzzer", "-i", "/seeds", "-o", "/output", "--", "target"]);
        assert_eq!(args.num_instances, 1);

        // Test explicit number of instances
        let args = parse_args(&[
            "fuzzer", "-i", "/seeds", "-o", "/output", "-j", "4", "--", "target",
        ]);
        assert_eq!(args.num_instances, 4);

        // Test with other options
        let args = parse_args(&[
            "fuzzer",
            "-i",
            "/seeds",
            "-o",
            "/output",
            "-j",
            "8",
            "-c",
            "block,edge",
            "--debug",
            "--",
            "target",
        ]);
        assert_eq!(args.num_instances, 8);
        assert_eq!(
            args.coverage_types,
            vec![String::from("block"), String::from("edge")]
        );
        assert_eq!(args.debug, true);
    }
}
