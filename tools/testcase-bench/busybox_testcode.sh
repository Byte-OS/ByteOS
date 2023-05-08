#!/bin/bash

# RST=result.txt
# if [ -f $RST ];then
# 	rm $RST
# fi
# touch $RST
# 
# echo "If the CMD runs incorrectly, return value will put in $RST" > $RST
# echo -e "Else nothing will put in $RST\n" >> $RST
# echo "TEST START" >> $RST

cat ./busybox_cmd.txt | while read line
do
	eval "busybox $line" >> run.txt
	RTN=$?
	if [[ $RTN == 0 || $line == "false" ]] ;then
		echo "testcase busybox $line success"
		echo "$line" >> passed.txt
	fi
done

cat run.txt

cat passed.txt

# echo "TEST END" >> $RST
