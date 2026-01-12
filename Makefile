BINARY = latc_llvm
EXTS = json mir lir opt.lir ll bc

all: build-all
	
build-all: build-llvm

build-llvm:
	cargo build --bin latc_llvm --release
	cp target/release/latc_llvm .

test: build
	@echo "Running tests..."
	cargo test

clean-all: clean
	cargo clean

clean:
	rm -f $(BINARY) lib/runtime.bc
	$(foreach ext, $(EXTS), find tests -type f -name "*.$(ext)" -delete;)

.PHONY:
	all build build-llvm test clean clean-all
