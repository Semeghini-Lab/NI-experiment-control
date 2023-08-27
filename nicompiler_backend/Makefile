export DYLD_LIBRARY_PATH=/System/Volumes/Data/Users/nicholaslyu/anaconda3/pkgs/python-3.10.9-hc0d8a6c_1/lib/:$DYLD_LIBRARY_PATH

# Default target (will run when you type `make` with no arguments)
all: test

# Define the test target
test:
	@echo "Running tests..."
	cargo test

run:
	@echo "Running the main function..."
	cargo run

# Define the export target
export:
	@echo "Building python and rust library ..."
	cargo build
	maturin develop

export_optimized:
	@echo "Building python and rust library with heavy optimization ..."
	cargo build --release
	maturin develop --release