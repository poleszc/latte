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
	rm -rf tests/good/*.ll tests/good/*.bc
	rm -rf tests/good/*.json
	rm -rf tests/extensions/arrays/*.json
	rm -rf tests/extensions/objects/*.json
	rm -rf tests/extensions/struct/*.json
	rm -rf tests/extensions/virtual/*.json
	rm -rf tests/bad/*.json
	rm -rf tests/bad/lexical/*.json

.PHONY:
	all build build-llvm test clean clean-all
