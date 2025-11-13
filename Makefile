PROJECT_NAME=instant-compiler

all: build-all
	
build-all: build-jvm build-llvm

build-jvm:
	cargo build --bin insc_jvm --release
	cp target/release/insc_jvm .

build-llvm:
	cargo build --bin insc_llvm --release
	cp target/release/insc_llvm .

test: build
	@echo "Running tests..."

clean-all: clean
	cargo clean

clean:
	echo "Cleaning up..."
	rm -rf insc_jvm insc_llvm
	rm -rf tests/in/*.j tests/in/*.class tests/in/*.ll tests/in/*.bc

.PHONY:
	all build build-jvm build-llvm test clean clean-all