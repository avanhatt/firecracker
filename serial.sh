# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

echo "Running RMC build"
set -e
rm -rf build
cd src/devices/src/virtio/

# New flag: -Z restrict_vtable_fn_ptrs
RUST_BACKTRACE=full RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc -Z restrict_vtable_fn_ptrs --cfg=rmc" RUSTC=rmc-rustc cargo build -j 1 --target x86_64-unknown-linux-gnu
cd ../../../..
cd build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 72 symtab2gb {} --out {.}.out &> symtab2gb.log

# # # New: combine restriction files from crate + dependencies into one
# # jq -s 'reduce .[] as $item ({}; . * $item)' *.fn_ptr_restrictions > restrictions.json

HARNESS=serial_recv_harness
# mkdir $HARNESS

# C file
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $HARNESS/a.out

goto-instrument --dump-c $HARNESS/a.out $HARNESS/a.c

goto-instrument $HARNESS/a.out $HARNESS/b.out 2>&1 | tee $HARNESS/goto-instrument.log
goto-instrument --dump-c $HARNESS/b.out $HARNESS/b.c

goto-instrument --remove-function-pointers $HARNESS/b.out $HARNESS/c.out 2>&1 | tee $HARNESS/goto-instrument-remove-function-pointers.log
goto-instrument --dump-c $HARNESS/c.out $HARNESS/c.c

goto-instrument --drop-unused-functions  $HARNESS/c.out $HARNESS/d.out 2>&1 | tee $HARNESS/slice.log
goto-instrument --dump-c $HARNESS/d.out $HARNESS/d.c

# echo "Running CBMC"
time cbmc $HARNESS/d.out --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 13 --unwinding-assertions --unwind 2 2>&1 | tee $HARNESS/cbmc.log
# time cbmc $HARNESS/d.out --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 11 --unwind 32 2>&1 | tee $HARNESS/cbmc.log

# time cbmc $HARNESS/e.out --object-bits 11 --unwinding-assertions --unwind 2 2>&1 | tee $HARNESS/cbmc.log

# time cbmc $HARNESS/d.out --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --object-bits 11 --unwinding-assertions --unwind 2 2>&1 | tee $HARNESS/cbmc.log
# [RMC] cmd: cbmc --bounds-check --pointer-check --pointer-primitive-check --conversion-check --div-by-zero-check --float-overflow-check --nan-check --pointer-overflow-check --signed-overflow-check --undefined-shift-check --unsigned-overflow-check --unwinding-assertions --function main src/test/rmc/DynTrait/main.goto

# # echo "Running just CBMC"
# # HARNESS=parse_harness
# # cd /home/ubuntu/firecracker/build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
# # # time cbmc $HARNESS/b.out --object-bits 11 --unwinding-assertions --unwind 2 2>&1 
# # time cbmc $HARNESS/b.out --object-bits 11 --bounds-check  --unwinding-assertions --unwind 2  2>&1 

