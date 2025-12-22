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
	rm -rf tests/good/extra/*.json
	rm -rf tests/good/official/*.json
	rm -rf tests/good/extensions/arrays/*.json
	rm -rf tests/good/extensions/objects/*.json
	rm -rf tests/good/extensions/struct/*.json
	rm -rf tests/good/extensions/virtual/*.json

	rm -rf tests/good/extra/*.mir.txt
	rm -rf tests/good/official/*.mir.txt
	rm -rf tests/good/extensions/arrays/*.mir.txt
	rm -rf tests/good/extensions/objects/*.mir.txt
	rm -rf tests/good/extensions/struct/*.mir.txt
	rm -rf tests/good/extensions/virtual/*.mir.txt

	rm -rf tests/good/extra/*.lir.txt
	rm -rf tests/good/official/*.lir.txt
	rm -rf tests/good/extensions/arrays/*.lir.txt
	rm -rf tests/good/extensions/objects/*.lir.txt
	rm -rf tests/good/extensions/struct/*.lir.txt
	rm -rf tests/good/extensions/virtual/*.lir.txt

	rm -rf tests/bad/*.json
	rm -rf tests/bad/lexical/*.json
	rm -rf tests/bad/official/*.json
	rm -rf tests/bad/semantic/basic/*.json
	rm -rf tests/bad/semantic/error_recovery/*.json
	rm -rf tests/bad/semantic/extensions/*.json

.PHONY:
	all build build-llvm test clean clean-all
