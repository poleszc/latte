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
	rm -rf tests/bad/runtime/*.json

	rm -rf tests/good/extra/*.mir
	rm -rf tests/good/official/*.mir
	rm -rf tests/good/extensions/arrays/*.mir
	rm -rf tests/good/extensions/objects/*.mir
	rm -rf tests/good/extensions/struct/*.mir
	rm -rf tests/good/extensions/virtual/*.mir
	rm -rf tests/bad/runtime/*.mir

	rm -rf tests/good/extra/*.lir
	rm -rf tests/good/official/*.lir
	rm -rf tests/good/extensions/arrays/*.lir
	rm -rf tests/good/extensions/objects/*.lir
	rm -rf tests/good/extensions/struct/*.lir
	rm -rf tests/good/extensions/virtual/*.lir
	rm -rf tests/bad/runtime/*.lir

	rm -rf tests/good/extra/*.ll
	rm -rf tests/good/official/*.ll
	rm -rf tests/good/extensions/arrays/*.ll
	rm -rf tests/good/extensions/objects/*.ll
	rm -rf tests/good/extensions/struct/*.ll
	rm -rf tests/good/extensions/virtual/*.ll
	rm -rf tests/bad/runtime/*.ll

	rm -rf tests/good/extra/*.bc
	rm -rf tests/good/official/*.bc
	rm -rf tests/good/extensions/arrays/*.bc
	rm -rf tests/good/extensions/objects/*.bc
	rm -rf tests/good/extensions/struct/*.bc
	rm -rf tests/good/extensions/virtual/*.bc
	rm -rf tests/bad/runtime/*.bc

	rm -rf tests/bad/*.json
	rm -rf tests/bad/lexical/*.json
	rm -rf tests/bad/official/*.json
	rm -rf tests/bad/semantic/basic/*.json
	rm -rf tests/bad/semantic/error_recovery/*.json
	rm -rf tests/bad/semantic/extensions/*.json

.PHONY:
	all build build-llvm test clean clean-all
