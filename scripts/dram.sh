#!/bin/bash

echo "Start benchmark"
# echo $(date)
# echo "R2R start"
# ./target/release/raplre --dir data --name dram01 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 30 --dump-dir data --dump-name dram01 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode r2r
# sleep 180 
#echo $(date)
#echo "dram 2 start"
#./target/release/raplre --dir data --name dram02 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 30 --dump-dir data --dump-name dram02 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(64)" --num-workers 32 --mode r2r
#echo "dram 2 end"
#sleep 120
#echo $(date)
#echo "dram 3 start"
#./target/release/raplre --dir data --name dram03 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 30 --dump-dir data --dump-name dram03 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(512)" --num-workers 32 --mode r2r
#echo "dram 3 end"
#sleep 120
# echo $(date)
# echo "R2S start"
# ./target/release/raplre --dir data --name dram04 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 30 --dump-dir data --dump-name dram04 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode r2s
# sleep 180
# echo $(date)
# echo "S2R start"
# ./target/release/raplre --dir data --name dram05 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 30 --dump-dir data --dump-name dram05 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode s2r
# sleep 180
# echo $(date)
# echo "S2S start"
# ./target/release/raplre --dir data --name dram06 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 30 --dump-dir data --dump-name dram06 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode s2s
# echo "dram 6 end"
#sleep 120
#echo $(date)
#echo "dram 7 start"
#./target/release/raplre --dir data --name dram07 benchmark --smooth ./target/release/raplre-bm-dram -- --time 30 --dump-dir data --dump-name dram07 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode r2r
#echo "dram 7 end"
#sleep 120
#echo $(date)
#echo "dram 8 start"
#./target/release/raplre --dir data --name dram08 benchmark --smooth ./target/release/raplre-bm-dram -- --time 30 --dump-dir data --dump-name dram08 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode r2s
#echo "dram 8 end"
#sleep 120
#echo $(date)
#echo "dram 9 start"
#./target/release/raplre --dir data --name dram09 benchmark --smooth ./target/release/raplre-bm-dram -- --time 30 --dump-dir data --dump-name dram09 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode s2r
#echo "dram 9 end"
#sleep 120
#echo $(date)
#echo "dram 10 start"
#./target/release/raplre --dir data --name dram10 benchmark --smooth ./target/release/raplre-bm-dram -- --time 30 --dump-dir data --dump-name dram10 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode s2s
#echo "dram 10 end"
#echo $(date)
# echo "Benchmark End!"

echo "Start benchmark"
echo $(date)
echo "R2R 1500 start"
./target/release/raplre --dir data --name dram11 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 15 --dump-dir data --dump-name dram11 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(1500)" --num-workers 32 --mode r2r
echo "R2R 1500 end"
sleep 60
echo $(date)
echo "R2R 512 start"
./target/release/raplre --dir data --name dram12 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 15 --dump-dir data --dump-name dram12 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(512)" --num-workers 32 --mode r2r
echo "R2R 512 end"
sleep 60
echo $(date)
echo "R2R 128 start"
./target/release/raplre --dir data --name dram13 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 15 --dump-dir data --dump-name dram13 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(128)" --num-workers 32 --mode r2r
echo "R2R 128 end"
sleep 60
echo $(date)
echo "R2R 64 start"
./target/release/raplre --dir data --name dram14 benchmark --smooth ./target/release/raplre-bm-dram -- --huge --page-size 2M --time 15 --dump-dir data --dump-name dram14 simple --buf-size 2048 --num-bufs 2048 --pkt-size-distri="constant(64)" --num-workers 32 --mode r2r
echo "R2R 64 end"
echo $(date)
echo "Benchmark End!"
