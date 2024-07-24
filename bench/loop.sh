#!/bin/sh


# ./target/release/run-rapl --dir data --name loop isolate -m 30
# ISOLATE_FILE=$(ls ./data/loop-isolate*.json)

./target/release/run-rapl --dir data --name loop_with_pause benchmark ./target/release/loop -- -d 1800 -p -l 0-31
OUPUT_FILE=$(ls ./data/loop_with_pause-benchmark-*.csv)
./target/release/run-rapl --dir data --name loop_with_pause extract $OUPUT_FILE
sleep 180
./target/release/run-rapl --dir data --name loop benchmark ./target/release/loop -- -d 1800 -l 0-31
OUPUT_FILE=$(ls ./data/loop-benchmark-*.csv)
./target/release/run-rapl --dir data --name loop extract $OUPUT_FILE