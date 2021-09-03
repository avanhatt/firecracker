rm -rf build
cd src/devices/src/virtio/
RUST_BACKTRACE=1 RUSTFLAGS="-Z trim-diagnostic-paths=no -Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
cd ../../../../build/cargo_target/x86_64-unknown-linux-gnu/debug/deps/
ls *.json | parallel -j 72 symtab2gb {} --out {.}.out

mkdir pass_proof_harness
goto-cc --function pass_proof_harness *.out -o pass_proof_harness/a.out
goto-instrument --drop-unused-functions --reachability-slice pass_proof_harness/a.out pass_proof_harness/b.out
cbmc pass_proof_harness/b.out --object-bits 11

mkdir fail_proof_harness
goto-cc --function fail_proof_harness *.out -o fail_proof_harness/a.out
goto-instrument --drop-unused-functions --reachability-slice fail_proof_harness/a.out fail_proof_harness/b.out
cbmc fail_proof_harness/b.out --object-bits 11

mkdir balloon_proof_harness
goto-cc --function balloon_proof_harness *.out -o balloon_proof_harness/a.out
goto-instrument --drop-unused-functions --reachability-slice balloon_proof_harness/a.out balloon_proof_harness/b.out
cbmc balloon_proof_harness/b.out --object-bits 11 --unwind 2 
