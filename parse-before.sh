# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

# New: combine restriction files from crate + dependencies into one
# Build link tool first: cargo build --release --manifest-path ~/rmc/src/tools/rmc-link-restrictions/Cargo.toml
OUTDIR=parse_proof_build
cd $OUTDIR
BEFORE=../before
rm -rf $BEFORE
mkdir $BEFORE

HARNESS=parse_harness

# Empty C file to pull in CBMC preprocessing
touch empty.c
goto-cc --function $HARNESS *.out empty.c -o $BEFORE/a.out

# WITHOUT function pointer restrictions
goto-instrument $RESTRICTIONS $BEFORE/a.out $BEFORE/b.out

goto-instrument --remove-function-pointers $BEFORE/b.out $BEFORE/c.out
goto-instrument --drop-unused-functions --reachability-slice $BEFORE/c.out $BEFORE/d.out 
goto-instrument --dump-c $BEFORE/d.out $BEFORE/d.c

echo "Running CBMC"

# fast:
time cbmc $BEFORE/d.out --object-bits 13 --unwinding-assertions --unwind 2 
