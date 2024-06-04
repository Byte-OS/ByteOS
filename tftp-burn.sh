#!/bin/sh
uftpd -n -o ftp=0,tftp=69 . &

pid=$!

minicom --color=on -D /dev/ttyUSB0

kill -9 $pid
