# Default target (will run when you type `make` with no arguments)
all: test

# Define the test target
test:
	@echo "Running tests..."
	cargo doc && cargo test

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
	maturin develop --release

publish_pypi:
	@echo "Updating tests, docs, exporting to python and publishing to pypi"
	git add . 
	cargo fmt && cargo doc
	git commit -am "pypi publish update" && git push 
	maturin develop --release
	maturin build --release --strip --manylinux off
	twine upload target/wheels/*

publish: 
	@echo "Updating tests, docs, and exporting to python, and publishing crate"
	git add .
	cargo fmt && cargo doc && cargo test 
	git commit -am "publish update" && git push 
	maturin develop --release
	cargo publish
