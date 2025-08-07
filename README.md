# Coverage Playground

A simple coverage-guided fuzzer designed for comparing different coverage metrics, including but not limited to: block coverage, edge coverage, and path coverage. The focus is on comparing coverage metrics, and so performance of the fuzzer is not our primary concern.

## Build

```bash
cargo build --release
```

The fuzzer binary locates at `target/release/dummy-fuzzer`.

## Usage

The fuzzer should be used with programs instrumented with [path-cov-instr](https://github.com/fEst1ck/path-cov-instr), which tracks the execution paths of a program. See [here](https://github.com/fEst1ck/coverage-playground-playground) for an example setup.

The usage is similar to AFL(++):

```bash
./dummy-fuzzer -i <input_seeds_dir> -o <output_dir> [-c <METRICS>] [-u <METRICS>] -- <target_program> [target_args...]
```

### Options

- `-i, --input-dir <DIR>`: Directory containing initial seed files
- `-o, --output-dir <DIR>`: Output directory where findings will be saved
- `-c, --coverage-type <METRICS>`: Comma-separated list of coverage metrics to track (block, edge, path)
- `-u, --use-coverage <METRICS>`: Comma-separated list of coverage metrics used to provide feedbacks to the fuzzer (block, edge, path)
- `--debug`: Enable debug mode (prints additional information)
- `-- <target_program> [args...]`: Target program and its arguments

### Input Modes

The fuzzer supports two modes of providing input to the target program:

1. **File Input**: Use `@@` in the target program's arguments to specify where the input file should be placed
   ```bash
   ./dummy-fuzzer -i seeds/ -o output/ -c edge,path -u edge -- ./target -f @@
   ```

2. **Stdin Input**: If no `@@` is specified, input will be provided via stdin
   ```bash
   ./dummy-fuzzer -i seeds/ -o output/ -c block -u block -- ./target
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
- `stats/coverage_*.json`: Coverage snapshots with timestamps in the filename

The logged information includes:
- Runtime duration
- Total executions
- Coverage count for each tracked metric
- Crash count
- Queue size
- Current fuzzing level

### Example

1. Create a seeds directory with initial inputs:
   ```bash
   mkdir -p seeds/
   echo "test" > seeds/test.txt
   ```

2. Run the fuzzer:
   ```bash
   # Track edge and path coverage, use edge coverage for feedback
   ./dummy-fuzzer -i seeds/ -o output/ -c edge,path -u edge -- ./target -f @@

   # Track all coverage types, use block and path coverage for feedback
   ./dummy-fuzzer -i seeds/ -o output/ -c block,edge,path -u block,path -- ./target -f @@

   # Track and use only block coverage
   ./dummy-fuzzer -i seeds/ -o output/ -c block -u block -- ./target
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