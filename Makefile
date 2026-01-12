all: build-all
	
build-all: build-llvm

build-llvm:
	cargo build --bin latc_llvm --release
	cp target/release/latc_llvm .

test: build
	@echo "Running tests..."

clean-all: clean
	cargo clean

clean:
	echo "Cleaning up..."
	rm -rf latc_llvm
	rm -rf tests/good/extra/*.json
	rm -rf tests/good/official/*.json
	rm -rf tests/good/extensions/arrays/*.json
	rm -rf tests/good/extensions/objects/*.json
	rm -rf tests/good/extensions/struct/*.json
	rm -rf tests/good/extensions/virtual/*.json
	rm -rf tests/good/opts/constant_folding_and_propagation/*.json
	rm -rf tests/good/opts/dead_code_elimination/*.json
	rm -rf tests/good/opts/branch_removal/*.json
	rm -rf tests/good/opts/gcse/*.json
	rm -rf tests/good/opts/copy_propagation/*.json
	rm -rf tests/good/opts/garbage_collection/*.json
	rm -rf tests/good/opts/tail_call/*.json
	rm -rf tests/bad/runtime/*.json
	rm -rf tests/bad/semantic/basic/*.json

	rm -rf tests/good/extra/*.mir
	rm -rf tests/good/official/*.mir
	rm -rf tests/good/extensions/arrays/*.mir
	rm -rf tests/good/extensions/objects/*.mir
	rm -rf tests/good/extensions/struct/*.mir
	rm -rf tests/good/extensions/virtual/*.mir
	rm -rf tests/good/opts/constant_folding_and_propagation/*.mir
	rm -rf tests/good/opts/dead_code_elimination/*.mir
	rm -rf tests/good/opts/branch_removal/*.mir
	rm -rf tests/good/opts/gcse/*.mir
	rm -rf tests/good/opts/copy_propagation/*.mir
	rm -rf tests/good/opts/garbage_collection/*.mir
	rm -rf tests/good/opts/tail_call/*.mir
	rm -rf tests/bad/runtime/*.mir
	rm -rf tests/bad/semantic/basic/*.mir

	rm -rf tests/good/extra/*.lir
	rm -rf tests/good/official/*.lir
	rm -rf tests/good/extensions/arrays/*.lir
	rm -rf tests/good/extensions/objects/*.lir
	rm -rf tests/good/extensions/struct/*.lir
	rm -rf tests/good/extensions/virtual/*.lir
	rm -rf tests/good/opts/constant_folding_and_propagation/*.lir
	rm -rf tests/good/opts/dead_code_elimination/*.lir
	rm -rf tests/good/opts/branch_removal/*.lir
	rm -rf tests/good/opts/gcse/*.lir
	rm -rf tests/good/opts/copy_propagation/*.lir
	rm -rf tests/good/opts/garbage_collection/*.lir
	rm -rf tests/good/opts/tail_call/*.lir
	rm -rf tests/bad/runtime/*.lir
	rm -rf tests/bad/semantic/basic/*.lir

	rm -rf tests/good/extra/*.opt.lir
	rm -rf tests/good/official/*.opt.lir
	rm -rf tests/good/extensions/arrays/*.opt.lir
	rm -rf tests/good/extensions/objects/*.opt.lir
	rm -rf tests/good/extensions/struct/*.opt.lir
	rm -rf tests/good/extensions/virtual/*.opt.lir
	rm -rf tests/good/opts/constant_folding_and_propagation/*.opt.lir
	rm -rf tests/good/opts/dead_code_elimination/*.opt.lir
	rm -rf tests/good/opts/branch_removal/*.opt.lir
	rm -rf tests/good/opts/gcse/*.opt.lir
	rm -rf tests/good/opts/copy_propagation/*.opt.lir
	rm -rf tests/good/opts/garbage_collection/*.opt.lir
	rm -rf tests/good/opts/tail_call/*.opt.lir
	rm -rf tests/bad/runtime/*.opt.lir
	rm -rf tests/bad/semantic/basic/*.opt.lir

	rm -rf tests/good/extra/*.ll
	rm -rf tests/good/official/*.ll
	rm -rf tests/good/extensions/arrays/*.ll
	rm -rf tests/good/extensions/objects/*.ll
	rm -rf tests/good/extensions/struct/*.ll
	rm -rf tests/good/extensions/virtual/*.ll
	rm -rf tests/good/opts/constant_folding_and_propagation/*.ll
	rm -rf tests/good/opts/dead_code_elimination/*.ll
	rm -rf tests/good/opts/branch_removal/*.ll
	rm -rf tests/good/opts/gcse/*.ll
	rm -rf tests/good/opts/copy_propagation/*.ll
	rm -rf tests/good/opts/garbage_collection/*.ll
	rm -rf tests/good/opts/tail_call/*.ll
	rm -rf tests/bad/runtime/*.ll
	rm -rf tests/bad/semantic/basic/*.ll

	rm -rf lib/runtime.bc 
	rm -rf tests/good/extra/*.bc
	rm -rf tests/good/official/*.bc
	rm -rf tests/good/extensions/arrays/*.bc
	rm -rf tests/good/extensions/objects/*.bc
	rm -rf tests/good/extensions/struct/*.bc
	rm -rf tests/good/extensions/virtual/*.bc
	rm -rf tests/good/opts/constant_folding_and_propagation/*.bc
	rm -rf tests/good/opts/dead_code_elimination/*.bc
	rm -rf tests/good/opts/branch_removal/*.bc
	rm -rf tests/good/opts/gcse/*.bc
	rm -rf tests/good/opts/copy_propagation/*.bc
	rm -rf tests/good/opts/garbage_collection/*.bc
	rm -rf tests/good/opts/tail_call/*.bc
	rm -rf tests/bad/runtime/*.bc
	rm -rf tests/bad/semantic/basic/*.bc

	rm -rf tests/bad/*.json
	rm -rf tests/bad/lexical/*.json
	rm -rf tests/bad/official/*.json
	rm -rf tests/bad/semantic/basic/*.json
	rm -rf tests/bad/semantic/error_recovery/*.json
	rm -rf tests/bad/semantic/extensions/*.json

.PHONY:
	all build build-llvm test clean clean-all
