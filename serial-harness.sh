# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

echo "Running RMC build: serial harness with restrictions"
set -e
rm -rf build
cd src/devices/src/virtio/

# New flag: -Z restrict_vtable_fn_ptrs
RUST_BACKTRACE=full RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc -Z restrict_vtable_fn_ptrs --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
cd ../../../..
cd build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 72 symtab2gb {} --out {.}.out &> symtab2gb.log

# New: combine restriction files from crate + dependencies into one
RESTRICTIONS=restrictions.json
rmc-link-restrictions . &> $RESTRICTIONS

HARNESS=serial_harness
mkdir $HARNESS

# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $HARNESS/a.out

goto-instrument --function-pointer-restrictions-file $RESTRICTIONS $HARNESS/a.out $HARNESS/b.out 
goto-instrument --remove-function-pointers $HARNESS/b.out $HARNESS/c.out 
goto-instrument --drop-unused-functions  $HARNESS/c.out $HARNESS/d.out
goto-instrument --dump-c $HARNESS/d.out $HARNESS/d.c

echo "Running CBMC"
time cbmc $HARNESS/d.out --trace --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 13 --unwinding-assertions --unwind 2