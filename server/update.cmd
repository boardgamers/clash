cargo update
rustup update
cargo update
cargo fmt
cd ..\client
cargo update
rustup update
cargo update
cargo fmt
cd ..\server
cargo nextest run
cargo clippy
cd ..\client
cargo clippy
cd ..\server
git stash
git fetch
git merge origin/main
git stash apply
