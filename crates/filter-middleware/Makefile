.PHONY: all setup build clean wit work

all: build

work: setup wit build

wit:
	@wit-deps update

setup:
	@rustup target add wasm32-wasip2

build:
	@cargo build --target wasm32-wasip2 --release

clean:
	@cargo clean
