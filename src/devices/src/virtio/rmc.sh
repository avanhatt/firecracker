rm -rf ../../../../build
RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
cd ../../../../build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
for j in *.json; do symtab2gb $j; mv a.out $j.out; done
touch empty.c
goto-cc --function rmc_compact_harness empty.c vm_memory-7b7da86bd22cbff0.json.out virtio_gen-*.json.out devices-*.json.out -o a.out
goto-instrument --drop-unused-functions a.out b.out
time cbmc b.out --object-bits 11 --unwind 1 --pointer-check --external-sat-solver ~/kissat/build/kissat
