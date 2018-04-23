#!/bin/sh

set -e

cd $(dirname $0)

cargo test
cargo build --release

cd image-to-level
cargo test
cd ..

mkdir -p output/sokoban
cp -a assets target/release/sokoban output/sokoban
strip output/sokoban/sokoban

cd output
tar cjf sokoban.tar.bz2 sokoban
