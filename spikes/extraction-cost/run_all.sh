#!/usr/bin/env bash
# Spike 5 runs on the prebuilt b11541 engine (CPU libs + CUDA libs from the b11538 cudart bundle).
P=~/AI/engines/_prebuilt; M=/home/omar/AI/models
export LD_LIBRARY_PATH=$P/mainline-b11541/llama-b11541:$P/mainline-b11538/cudart-llama-b11538-bin-ubuntu-cuda-13.4-x64
E=$P/mainline-b11541/llama-b11541/llama-server
python3 -I bench.py LFM2.5-2.6B 6000 -- $E -m $M/lfm25-2.6b/LFM2.5-2.6B-Q4_K_M.gguf -ngl 99 -fa on -ctk q8_0 -ctv q8_0 -t 6 -np 1 -ub 1024 -c 16384 --cache-ram 0
python3 -I bench.py Spark-X2.5-4B 6000 -- $E -m $M/spark-x2.5-4b/Spark-X2.5-4B-Q4_K_M.gguf -ngl 99 -fa on -ctk q8_0 -ctv q8_0 -t 8 -np 1 -ub 512 -c 16384 --jinja --cache-ram 0
python3 -I bench.py Ornith-1.5-35B-A3B 3000 -- $E -m $M/ornith-15-35b-nanoplus/Ornith-1.5-35B-A3B.APEX-I-NanoPlus.gguf -ngl 99 --n-cpu-moe 40 -fa on -ctk q8_0 -ctv q8_0 -t 6 -b 2048 -ub 256 -np 1 -c 16384 --jinja --cache-ram 0
