.PHONY: default hc test check clean

default: hc

# cargo decides whether a rebuild is needed; the copy puts hc where
# hascheevos.sh looks for it
hc:
	cargo build --release
	install -m 755 target/release/hc bin/hc

# T filters the tests by name: make test T=<name>
test:
	cargo test $(T)

# everything that must pass before work is done; the bats tests call
# bin/hc, so hc comes before them
check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test
	$(MAKE) hc
	bats tests/bats

clean:
	-rm -f bin/hc
