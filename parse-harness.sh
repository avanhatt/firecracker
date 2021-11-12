# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

echo "Running RMC build: parse harness with restrictions"
set -e
rm -rf build
cd src/devices/src/virtio/

# # New flag: -Z restrict_vtable_fn_ptrs
FLAGS=$(rmc-rustc --rmc-flags)
FLAGS+=" -Z restrict_vtable_fn_ptrs"
RUST_BACKTRACE=1 RUSTFLAGS=$FLAGS RUSTC=$(rmc-rustc --rmc-path) cargo build --target x86_64-unknown-linux-gnu -j 16
# # RUST_BACKTRACE=full RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc -Z restrict_vtable_fn_ptrs --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
cd ../../../..
cd build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/

# # New: combine restriction files from crate + dependencies into one
RESTRICTIONS=restrictions.json
cargo run --release --manifest-path ~/rmc/src/tools/rmc-link-restrictions/Cargo.toml . &> $RESTRICTIONS

HARNESS=parse_harness
mkdir $HARNESS

# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $HARNESS/a.out

# # No function pointer restrictions
# goto-instrument $HARNESS/a.out $HARNESS/b.out

# With function pointer restrictions
goto-instrument --function-pointer-restrictions-file $RESTRICTIONS $HARNESS/a.out $HARNESS/b.out

goto-instrument --remove-function-pointers $HARNESS/b.out $HARNESS/c.out
goto-instrument --drop-unused-functions --reachability-slice $HARNESS/c.out $HARNESS/d.out 
goto-instrument --dump-c $HARNESS/d.out $HARNESS/d.c

echo "Running CBMC"

# fast:
time cbmc $HARNESS/d.out --object-bits 13 --unwinding-assertions --unwind 2 
