echo "KANI: Running build: serial harness WITH restrictions"
set -e
rm -rf build
cd src/devices/src/virtio/

# New flag: --restrict-vtable-fn-ptrs
export RUSTC_LOG=error
export KANIFLAGS="--goto-c --restrict-vtable-fn-ptrs"
export RUSTFLAGS="--cfg=kani --kani-flags"
export RUSTC="kani-rustc"
cargo build --target x86_64-unknown-linux-gnu

echo "KANI: Build done"

echo "KANI: Getting Goto-C from symbol tables"
cd ../../../../build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 16 symtab2gb {} --out {.}.out &> /dev/null || :

# Combine restriction files from crate + dependencies into one
echo "KANI: Linking restrictions"
RESTRICTIONS=restrictions.json
/rmc/target/release/kani-link-restrictions . $RESTRICTIONS

HARNESS=serial_harness
mkdir $HARNESS

echo "KANI: Processing Goto-C"
# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $HARNESS/a.out

goto-instrument --function-pointer-restrictions-file $RESTRICTIONS $HARNESS/a.out $HARNESS/b.out 
goto-instrument --remove-function-pointers $HARNESS/b.out $HARNESS/c.out 
goto-instrument --drop-unused-functions  $HARNESS/c.out $HARNESS/d.out
goto-instrument --dump-c $HARNESS/d.out $HARNESS/d.c

echo "KANI: Running CBMC"
time cbmc $HARNESS/d.out --trace --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 13 --unwinding-assertions --unwind 2
