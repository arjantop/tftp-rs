#!/usr/bin/env bash

mkdir -p fixtures && cd fixtures

echo "Creating fixtures ..."

dd if=/dev/urandom of=001mb bs=1048576 count=1
dd if=/dev/urandom of=010mb bs=1048576 count=10
dd if=/dev/urandom of=050mb bs=1048576 count=50
dd if=/dev/urandom of=100mb bs=1048576 count=100
dd if=/dev/urandom of=250mb bs=1048576 count=250
