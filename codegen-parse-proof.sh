# Note, this version of the script requires modifications to both RMC and CBMC
# RMC PR:  https://github.com/model-checking/rmc/pull/536
# CBMC PR: https://github.com/diffblue/cbmc/pull/6376

CODEGEN=build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
OUTDIR=parse_proof_build

echo "Running code generation for parse proof harness"
set -e
rm -rf build
rm -rf $OUTDIR
cd src/devices/src/virtio/

# New flag: -Z restrict_vtable_fn_ptrs
FLAGS=$(rmc-rustc --rmc-flags)
FLAGS+=" -Z restrict_vtable_fn_ptrs --cfg=rmc "
RUST_BACKTRACE=1 RUSTFLAGS=$FLAGS RUSTC=$(rmc-rustc --rmc-path) cargo build --target x86_64-unknown-linux-gnu -j 16 &> log.txt
cd ../../../..
mkdir $OUTDIR
cp $CODEGEN/* $OUTDIR
ls $OUTDIR/*.symtab.json | parallel -j 72 symtab2gb {} --out {.}.out &> symtab2gb.log
echo "Translation to CBMC successful"