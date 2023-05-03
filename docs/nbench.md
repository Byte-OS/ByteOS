# nbench

> 利用 nbench 进行性能测试

## ByteOS (qemu-system-riscv64)
  ____        _        ____   _____ 
 |  _ \      | |      / __ \ / ____|
 | |_) |_   _| |_ ___| |  | | (___  
 |  _ <| | | | __/ _ \ |  | |\___ \ 
 | |_) | |_| | ||  __/ |__| |____) |
 |____/ \__, |\__\___|\____/|_____/ 
         __/ |                      
        |___/                       


BYTEmark* Native Mode Benchmark ver. 2 (10/95)
Index-split by Andrew D. Balsa (11/97)
Linux/Unix* port by Uwe F. Mayer (12/96,11/97)

TEST                : Iterations/sec.  : Old Index   : New Index
                    :                  : Pentium 90* : AMD K6/233*
--------------------:------------------:-------------:------------
NUMERIC SORT        :             879  :      22.54  :       7.40
STRING SORT         :          17.575  :       7.85  :       1.22
BITFIELD            :      4.2969e+08  :      73.71  :      15.40
FP EMULATION        :          175.84  :      84.38  :      19.47
FOURIER             :          8865.2  :      10.08  :       5.66
ASSIGNMENT          :           29.84  :     113.55  :      29.45
IDEA                :          4659.2  :      71.26  :      21.16
HUFFMAN             :          1886.4  :      52.31  :      16.70

## Linux (qemu-riscv64)

BYTEmark* Native Mode Benchmark ver. 2 (10/95)
Index-split by Andrew D. Balsa (11/97)
Linux/Unix* port by Uwe F. Mayer (12/96,11/97)

TEST                : Iterations/sec.  : Old Index   : New Index
                    :                  : Pentium 90* : AMD K6/233*
--------------------:------------------:-------------:------------
NUMERIC SORT        :          1130.1  :      28.98  :       9.52
STRING SORT         :           36.48  :      16.30  :       2.52
BITFIELD            :      7.1698e+08  :     122.99  :      25.69
FP EMULATION        :          200.99  :      96.44  :      22.25
FOURIER             :           10046  :      11.43  :       6.42
ASSIGNMENT          :          36.577  :     139.18  :      36.10
IDEA                :          5445.9  :      83.29  :      24.73
HUFFMAN             :            2791  :      77.39  :      24.71