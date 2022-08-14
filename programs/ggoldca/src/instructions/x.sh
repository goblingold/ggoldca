#!/bin/bash

DIR="$(pwd)"
IDL="$HOME/GoblinGold/ggoldca/target/idl/ggoldca.json"
SDK="$HOME/GoblinGold/ggoldca-sdk/src/idl/ggoldca.json"

#if ! cmp -s $IDL $SDK; then
#  cp $IDL $SDK
#  cd ${SDK%/*}
#  yarn build
#  cd $DIR
#fi


cd $HOME/GoblinGold/ggoldca/tests/
$HOME/skrrb/github/anchor/target/release/anchor test --run ggolca -- --features test

cat ~/GoblinGold/ggoldca/.anchor/program-logs/ECzqPRCK7S7jXeNWoc3QrYH6yWQkcQGpGR2RWqRQ9e9P.ggoldca.log
