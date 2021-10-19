echo "Running RMC build: serial harness with no restrictions"
set -e
rm -rf build
cd src/devices/src/virtio/

RUST_BACKTRACE=full RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
cd ../../../..
cd build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 72 symtab2gb {} --out {.}.out &> symtab2gb.log

HARNESS=serial_harness
mkdir $HARNESS

# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $HARNESS/a.out

goto-instrument $HARNESS/a.out $HARNESS/b.out
goto-instrument --remove-function-pointers $HARNESS/b.out $HARNESS/c.out 
goto-instrument --drop-unused-functions  $HARNESS/c.out $HARNESS/d.out 
echo "Running CBMC"
time cbmc $HARNESS/d.out --trace --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 13 --unwinding-assertions --unwind 2