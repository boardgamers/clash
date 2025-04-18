cargo update
rustup update
cargo update
cargo clean
cargo fmt
cd ..\client
cargo update
rustup update
cargo update
cargo clean
cargo fmt
cd ..
git stash
git fetch
git merge origin/main
git stash apply
cd ..\server
cargo nextest run
