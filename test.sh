# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

echo "Running just RMC build and CBMC"
set -e
rm -rf build
cd src/devices/src/virtio/

# New flag: -Z restrict_vtable_fn_ptrs
RUST_BACKTRACE=full RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc -Z restrict_vtable_fn_ptrs --cfg=rmc" RUSTC=rmc-rustc cargo build  -j 1 --target x86_64-unknown-linux-gnu
cd ../../../..
cd build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 72 symtab2gb {} --out {.}.out &> symtab2gb.log

# New: combine restriction files from crate + dependencies into one
jq -s 'reduce .[] as $item ({}; . * $item)' *.fn_ptr_restrictions > restrictions.json

HARNESS=parse_harness
mkdir $HARNESS
goto-cc --function $HARNESS *.out -o $HARNESS/a.out

# New flag: --function-pointer-restrictions-file all_restrictions.json 
goto-instrument  --function-pointer-restrictions-file restrictions.json --generate-function-body mmap --generate-function-body-options assert-false --drop-unused-functions --reachability-slice $HARNESS/a.out $HARNESS/b.out 2>&1 | tee $HARNESS/goto-instrument.log
goto-instrument --remove-function-pointers $HARNESS/b.out $HARNESS/c.out 2>&1 | tee $HARNESS/goto-instrument-remove-function-pointers.log
goto-instrument --dump-c $HARNESS/c.out $HARNESS/genc.c
time cbmc --trace $HARNESS/b.out --object-bits 11 --unwind 1 --unwinding-assertions 2>&1 | tee $HARNESS/cbmc.log

# # echo "Running just CBMC"
# HARNESS=parse_harness
# cd /home/ubuntu/firecracker/build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
# time cbmc $HARNESS/b.out --object-bits 11 --unwind 2
