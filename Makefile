all: build-all
	
build-all: build-llvm

build-llvm:
	cargo build --bin insc_llvm --release
	cp target/release/insc_llvm .

test: build
	@echo "Running tests..."

clean-all: clean
	cargo clean

clean:
	echo "Cleaning up..."
	rm -rf insc_llvm
	rm -rf tests/in/*.ll tests/in/*.bc

.PHONY:
	all build build-llvm test clean clean-all