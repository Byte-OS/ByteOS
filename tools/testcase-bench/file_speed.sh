echo "File Speed TEST"
# busybox touch /var/tmp/hello
# ./lmbench_all lat_proc -P 1 shell
./lmbench_all lmdd label="File /var/tmp/XXX write bandwidth:" of=/var/tmp/XXX move=2m fsync=1 print=3
