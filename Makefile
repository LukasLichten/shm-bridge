.phony: all build release acc ac rf2 clear clean-up help

all: build

build: 
	cargo build

release:
	cargo build --release

ac: build
	protontricks-launch --appid 244210 ./target/x86_64-pc-windows-gnu/debug/shm-bridge.exe -m acpmf_crewchief acpmf_static acpmf_physics acpmf_graphics -s 15660 2048 2048 2048

acc: build
	protontricks-launch --appid 805550 ./target/x86_64-pc-windows-gnu/debug/shm-bridge.exe -m acpmf_crewchief acpmf_static acpmf_physics acpmf_graphics -s 15660 2048 2048 2048
	
rf2: build
	@echo "TODO"

clear: clean-up
	cargo clean

clean-up:
	cargo run -- -m acpmf_crewchief acpmf_static acpmf_physics acpmf_graphics --clean-up

help:
	@echo "Builds and test the shm-bridge"
	@echo "make:           Builds"
	@echo "make release:   Builds in release mode"
	@echo "make ac:        Build and Run Memory Maps in the AC prefix"
	@echo "make acc:       Build and Run Memory Maps in the ACC prefix"
	@echo "make rf2:       Build and Run Memory Maps in the rF2 prefix"
	@echo "make clean-up:  Removes Stale Memory Maps"
	@echo "make clear:     Clean-up but also runs Cargo Clean"
	@echo "make help:      This Printout"
