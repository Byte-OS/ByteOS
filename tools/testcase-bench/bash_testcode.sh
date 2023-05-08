#!/bin/bash

# 测试创建文件
echo -n "Bash Test TOUCH FILE " > log
touch exists.file
if [ -f "exists.file" ] 
then
    echo "PASS" >> log
else
    echo "FAILED" >> log
fi
cat log
rm log

# 测试读取文件
echo -n "Bash Test READ FILE " > log
echo "Hello" >> 1.txt
var=$(cat 1.txt)
if [ $var = "Hello" ]; then
    echo "PASS" >> log
else
    echo "FAILED" >> log
fi
cat log
rm log

# 测试循环
echo -n "Bash Test LOOP " > log
sum=0
for i in `seq 1 100`
do
    sum=$(($sum + $i))
done
if [ $sum = 5050 ]; then
    echo "PASS" >> log
else
    echo "FAILED" >> log
fi
cat log
rm log

# 测试删除文件
echo -n "Bash Test DELETE FILE " > log
rm exists.file
if [ -f "exists.file" ] 
then
    echo "FAILED" >> log
else
    echo "PASS" >> log
fi
cat log
rm log

# 测试执行命令
echo -n "Bash Test DELETE EVAL COMMAND " > log
eval "touch exists.file"
if [ -f "exists.file" ] 
then
    echo "PASS" >> log
else
    echo "FAILED" >> log
fi
cat log
rm log


# 测试执行运算
echo -n "Bash Test EXPR" > log
num=`expr 1 + 1`
if [ $num = 2 ] 
then
    echo "PASS" >> log
else
    echo "FAILED" >> log
fi
cat log
rm log


# 测试管道
echo -n "Bash Test PIPE" > log
str=$(echo "hello" | cut -c 3-)
if [ $str = 'llo' ] 
then
    echo "PASS" >> log
else
    echo "FAILED" >> log
fi
cat log
rm log