#!/bin/sh

set -e

cargo build --release

cd $(dirname $0)
mkdir -p output/sokoban
cp -a assets target/release/sokoban output/sokoban
strip output/sokoban/sokoban

cd output
tar cjf sokoban.tar.bz2 sokoban
