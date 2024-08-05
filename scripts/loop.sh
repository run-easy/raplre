#!/bin/sh

# ./target/release/run-rapl --dir data --name loop isolate -m 30
# ISOLATE_FILE=$(ls ./data/loop-isolate*.json)


./target/release/raplre --dir data --name loop_with_pause benchmark ./target/release/raplre-bm-loop -- -d 7200 -p -l 0-31
#OUPUT_FILE=$(ls ./data/loop_with_pause-benchmark-*.csv)
#./target/release/raplre --dir data --name loop_with_pause extract --smooth --alpha $OUPUT_FILE
sleep 180
./target/release/raplre --dir data --name loop benchmark ./target/release/raplre-bm-loop -- -d 7200 -l 0-31
#OUPUT_FILE=$(ls ./data/loop-benchmark-*.csv)
#./target/release/raplre --dir data --name loop extract --smooth --alpha 0.02 $OUPUT_FILE
