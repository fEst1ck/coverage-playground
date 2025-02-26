# Dummy Fuzzer(WIP)

A simple coverage-guided fuzzer for comparing different coverage metrics. The fuzzer should be used with programs instrumented with https://github.com/fEst1ck/path-cov-instr.

## Building

```bash
cargo build --release
```

The fuzzer binary locates at `target/release/dummy-fuzzer`.

## Usage

```bash
./dummy-fuzzer -i <input_seeds_dir> -o <output_dir> [-c <coverage_type>] -- <target_program> [target_args...]
```

### Arguments

- `-i, --input-dir <DIR>`: Directory containing initial seed files
- `-o, --output-dir <DIR>`: Output directory where findings will be saved
- `-c, --coverage-type <TYPE>`: Coverage type to use (default: "block")
  - Supported types: "block", "edge", "path"
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

The fuzzer creates two directories under the specified output directory:

- `queue/`: Contains test cases that trigger new coverage
- `crashes/`: Contains inputs that caused the target to crash

### Example

1. Create a seeds directory with initial inputs:
   ```bash
   mkdir -p seeds/
   echo "test" > seeds/test.txt
   ```

2. Run the fuzzer:
   ```bash
   # For a program that reads from file
   ./dummy-fuzzer -i seeds/ -o output/ -- ./target -f @@

   # For a program that reads from stdin
   ./dummy-fuzzer -i seeds/ -o output/ -- ./target
   ```

### Crash Detection

The fuzzer detects and saves inputs that cause the following signals:
- SIGSEGV (11): Segmentation fault
- SIGABRT (6): Abort
- SIGBUS (7): Bus error

Other signals are logged but don't trigger crash saving.

## Development

The fuzzer uses a simple mutation strategy:
1. Pick a random byte
2. Either flip a random bit or replace with a random byte

Coverage is tracked using shared memory at `/tmp/coverage_shm.bin`. 