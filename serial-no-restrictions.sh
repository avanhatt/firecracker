echo "KANI: Running build: serial harness WITHOUT restrictions"
set -e
rm -rf build
cd src/devices/src/virtio/

# New flag: -Z restrict_vtable_fn_ptrs
export RUSTC_LOG=error
export KANIFLAGS="--goto-c"
export RUSTFLAGS="--cfg=kani --kani-flags"
export RUSTC="kani-rustc"
cargo build --target x86_64-unknown-linux-gnu

echo "KANI: Build done"

echo "KANI: Getting Goto-C from symbol tables"
cd ../../../../build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 16 symtab2gb {} --out {.}.out &> /dev/null || :

HARNESS=serial_harness
mkdir $HARNESS

echo "KANI: Processing Goto-C"
# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $HARNESS/a.out

goto-instrument --remove-function-pointers $HARNESS/a.out $HARNESS/b.out 
goto-instrument --drop-unused-functions  $HARNESS/b.out $HARNESS/c.out
goto-instrument --dump-c $HARNESS/c.out $HARNESS/c.c

echo "KANI: Running CBMC"
time cbmc $HARNESS/c.out --trace --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 13 --unwinding-assertions --unwind 2