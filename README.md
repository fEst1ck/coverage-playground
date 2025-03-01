# Dummy Fuzzer

A simple coverage-guided fuzzer designed for comparing different coverage metrics: block coverage, edge coverage, and path coverage. Our fuzzer can track all three coverages at the same time, and uses one of them to guide the fuzzing process. Our goal is to compare the effectiveness of different coverage metrics, and so performance of the fuzzer is not our concern.

The fuzzer should be used with programs instrumented with https://github.com/fEst1ck/path-cov-instr, which tracks the execution paths of a program.

## Build

```bash
cargo build --release
```

The fuzzer binary locates at `target/release/dummy-fuzzer`.

## Usage

The usage is similar to AFL(++):

```bash
./dummy-fuzzer -i <input_seeds_dir> -o <output_dir> [-c <coverage_type>] [-a] -- <target_program> [target_args...]
```

### Options

- `-i, --input-dir <DIR>`: Directory containing initial seed files
- `-o, --output-dir <DIR>`: Output directory where findings will be saved
- `-c, --coverage-type <TYPE>`: Coverage type to use (default: "block")
  - Supported types: "block", "edge", "path"
- `-a, --all-coverage`: Enable tracking of all coverage types simultaneously
- `-- <target_program> [args...]`: Target program and its arguments

### Input Modes

The fuzzer supports two modes of providing input to the target program:

1. **File Input**: Use `@@` in the target program's arguments to specify where the input file should be placed
   ```bash
   ./dummy-fuzzer -i seeds/ -o output/ -- ./target -f @@
   ```

2. **Stdin Input**: If no `@@` is specified, input will be provided via stdin
   ```bash
   ./dummy-fuzzer -i seeds/ -o output/ -- ./target
   ```

### Output Structure

The fuzzer creates the following directories under the specified output directory:

- `queue/`: Contains test cases that trigger new coverage
- `crashes/`: Contains inputs that caused the target to crash
- `stats/`: Contains logging information and statistics

Additionally, the fuzzer creates:
- `command.txt`: Records the exact command used to start the fuzzer and the start time

### Logging and Statistics

The fuzzer automatically logs its state every 30 seconds to provide insights into the fuzzing progress:

- `stats/fuzzer_log.json`: Contains detailed state information at each logging interval
- `stats/progress_data.csv`: CSV file for easy data analysis and visualization

The logged information includes:
- Runtime duration
- Total executions
- Coverage count (varies based on coverage type)
- Crash count
- Queue size
- Current fuzzing level

When using the `-a` (all-coverage) flag, the coverage information includes separate counts for block, edge, and path coverage.

### Example

1. Create a seeds directory with initial inputs:
   ```bash
   mkdir -p seeds/
   echo "test" > seeds/test.txt
   ```

2. Run the fuzzer:
   ```bash
   # For a program that reads from file with block coverage
   ./dummy-fuzzer -i seeds/ -o output/ -c block -- ./target -f @@

   # For a program that reads from stdin with path coverage
   ./dummy-fuzzer -i seeds/ -o output/ -c path -- ./target
   
   # For a program with all coverage types enabled
   ./dummy-fuzzer -i seeds/ -o output/ -a -- ./target -f @@
   ```

### Crash Detection

The fuzzer detects and saves inputs that cause the following signals:
- SIGSEGV (11): Segmentation fault
- SIGABRT (6): Abort
- SIGBUS (7): Bus error

Other signals are logged but don't trigger crash saving.

## Development

The fuzzer uses multiple mutation strategies:
1. Bit flip (30% chance)
2. Byte replacement (20% chance)
3. Delete consecutive bytes (25% chance)
4. Clone/insert bytes (25% chance)

Coverage, i.e., execution path, is tracked using shared memory at `/tmp/coverage_shm.bin`. 