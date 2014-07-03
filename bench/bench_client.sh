#!/usr/bin/env bash

TFTPD_BIN=/usr/bin/in.tftpd
TFTP_BIN=/usr/bin/tftp

if [ ! -d fixtures ]; then
    echo "Run create_fixtures.sh to generate fixtures"
    exit 1
fi

for file in `ls fixtures`
do
    echo "Getting file: [0;32m$file[0m"
    echo "Using: [0;31mtftp-hpa[0m"
    sleep 1
    /usr/bin/time "$TFTP_BIN" -v 127.0.0.1 69 -m octet -c get "$file" /tmp/testfile
    echo "Using: [0;34mtftp-rs[0m"
    sleep 1
    /usr/bin/time ../target/get "$file"
done
