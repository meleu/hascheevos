.PHONY: default hc clean

default: hc

# cargo decides whether a rebuild is needed; the copy puts hc where
# hascheevos.sh looks for it
hc:
	cargo build --release
	install -m 755 target/release/hc bin/hc

clean:
	-rm -f bin/hc
