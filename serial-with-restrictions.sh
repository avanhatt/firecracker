# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

# New: combine restriction files from crate + dependencies into one
# Build link tool first: cargo build --release --manifest-path ~/rmc/src/tools/rmc-link-restrictions/Cargo.toml
OUTDIR=serial_proof_build
cd $OUTDIR
WITH_RESTRICTIONS=../with_restrictions
rm -rf $WITH_RESTRICTIONS
mkdir $WITH_RESTRICTIONS

RESTRICTIONS=restrictions-linked.json
~/rmc/target/release/rmc-link-restrictions . $WITH_RESTRICTIONS/$RESTRICTIONS

HARNESS=serial_offset_harness

# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $WITH_RESTRICTIONS/a.out

# With function pointer restrictions
goto-instrument --function-pointer-restrictions-file $WITH_RESTRICTIONS/$RESTRICTIONS $WITH_RESTRICTIONS/a.out $WITH_RESTRICTIONS/b.out

goto-instrument --remove-function-pointers $WITH_RESTRICTIONS/b.out $WITH_RESTRICTIONS/c.out
goto-instrument --drop-unused-functions --reachability-slice $WITH_RESTRICTIONS/c.out $WITH_RESTRICTIONS/d.out 
goto-instrument --dump-c $WITH_RESTRICTIONS/d.out $WITH_RESTRICTIONS/d.c

echo "Running CBMC"

# fast:
time cbmc $WITH_RESTRICTIONS/d.out --object-bits 13 --unwinding-assertions --unwind 2 